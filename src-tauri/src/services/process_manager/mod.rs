pub(crate) mod types;
mod hwnd;
mod input;
mod monitoring;
mod hooks;
mod attach;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tauri::Emitter;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::error::AppError;
use crate::models::terminal::Instance;
use crate::services::debug_log::DebugLog;
use crate::services::reminder::Reminder;

use types::*;
use hwnd::*;
use input::send_ctrl_shift_w;

/// 터미널 프로세스의 생명주기를 관리하는 서비스.
/// wt.exe 실행, HWND 추적, 포커스 전환, 종료 감지, IO 활동 모니터링을 담당한다.
pub(crate) struct Manager {
    terminals: Arc<Mutex<HashMap<String, ManagedTerminal>>>,
    debug_log: Arc<DebugLog>,
    reminder: Option<Arc<Mutex<Reminder>>>,
}

impl Manager {
    /// 터미널 맵의 Mutex를 잠그고 Guard를 반환한다. 잠금 실패 시 AppError를 반환.
    fn lock_terminals(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, ManagedTerminal>>, AppError> {
        self.terminals.lock()
            .map_err(|_| AppError::Validation("Terminal lock poisoned".to_string()))
    }

    /// 빈 터미널 목록으로 새 Manager를 생성한다.
    pub(crate) fn new(debug_log: Arc<DebugLog>) -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
            debug_log,
            reminder: None,
        }
    }

    /// 리마인더 서비스를 연결한다. close() 및 모니터링 루프에서 사용.
    pub(crate) fn set_reminder(&mut self, reminder: Arc<Mutex<Reminder>>) {
        self.reminder = Some(reminder);
    }

    /// 지정된 프로젝트 경로에서 Windows Terminal을 실행하고 인스턴스를 등록한다.
    pub(crate) fn launch(
        &self,
        project_id: &str,
        project_name: &str,
        project_path: &str,
        language: &str,
    ) -> Result<Instance, AppError> {
        // project_id 검증: 최대 255자, 영숫자·공백·'-'·'_'·'.' 만 허용
        if project_id.len() > MAX_NAME_LENGTH {
            return Err(AppError::Validation("project_id가 너무 깁니다 (최대 255자)".to_string()));
        }
        if !project_id.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.')) {
            return Err(AppError::Validation(
                "project_id에 허용되지 않는 문자가 포함되어 있습니다".to_string()
            ));
        }

        // project_name 검증: 최대 255자, 영숫자·공백·'-'·'_'·'.' 만 허용
        if project_name.len() > MAX_NAME_LENGTH {
            return Err(AppError::Validation("project_name이 너무 깁니다 (최대 255자)".to_string()));
        }
        if !project_name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.')) {
            return Err(AppError::Validation(
                "project_name에 허용되지 않는 문자가 포함되어 있습니다".to_string()
            ));
        }

        let path = std::path::Path::new(project_path);
        match std::fs::metadata(&path) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(AppError::Validation(format!("Not a directory: {}", project_path))),
            Err(e) => return Err(AppError::Io(e)),
        }

        let title = format!("termi: {}", project_name);
        let window_name = format!("termi_{}", project_id);
        let child = std::process::Command::new(ALLOWED_TERMINAL_EXE)
            .args(["-w", &window_name, "nt", "--title", &title, "-d", project_path])
            .spawn()
            .map_err(AppError::Io)?;

        let instance = {
            let mut terminals = self.lock_terminals()?;
            let (terminal_number, tab_index) = next_terminal_number_and_tab(&terminals, project_id);
            let inst = create_instance(project_id, project_name, project_path, terminal_number, language, false);
            terminals.insert(inst.id.clone(), ManagedTerminal {
                instance: inst.clone(),
                hwnd: None,
                tab_index,
                runtime_id: None,
                title_tracking: false,
                pending_active_since: None,
                child: Some(child),
                hook_session_id: None,
                tab_renamed: false,
                attached: false,
            });
            inst
        };

        // 백그라운드에서 HWND 탐색
        let terminal_id = instance.id.clone();
        let terminal_name_for_log = instance.terminal_name.clone();
        let title_prefix = title;
        let terminals_clone = Arc::clone(&self.terminals);
        let debug_log_clone = Arc::clone(&self.debug_log);
        debug_log_clone.push("HWND", format!("{}: HWND search started (title=\"{}\")", terminal_name_for_log, title_prefix));
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(HWND_INITIAL_DELAY_MS)).await;
            for attempt in 1..=HWND_SEARCH_MAX_RETRIES {
                if let Some(hwnd) = find_terminal_hwnd(&title_prefix) {
                    debug_log_clone.push("HWND", format!("{}: HWND found = {} (attempt #{})", terminal_name_for_log, hwnd, attempt));
                    if let Ok(mut terminals) = terminals_clone.lock() {
                        if let Some(mt) = terminals.get_mut(&terminal_id) {
                            mt.hwnd = Some(hwnd);
                        }
                    }
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(HWND_SEARCH_INTERVAL_MS)).await;
            }
            debug_log_clone.push("HWND", format!("{}: HWND search failed! (gave up after {} retries)", terminal_name_for_log, HWND_SEARCH_MAX_RETRIES));
            eprintln!("HWND not found for terminal: {}", terminal_id);
        });

        Ok(instance)
    }

    /// 현재 등록된 모든 터미널 인스턴스 목록을 반환한다.
    pub(crate) fn get_all(&self) -> Result<Vec<Instance>, AppError> {
        let terminals = self.lock_terminals()?;
        Ok(terminals.values().map(|mt| mt.instance.clone()).collect())
    }

    /// 지정된 터미널의 Windows Terminal 창을 포그라운드로 이동하고 해당 탭을 활성화한다.
    /// 포커스 시 활성 상태로 전환하고 idle 타이머를 리셋한다.
    pub(crate) fn focus(&self, terminal_id: &str) -> Result<Instance, AppError> {
        let mut terminals = self.lock_terminals()?;

        let mt = terminals.get_mut(terminal_id)
            .ok_or_else(|| AppError::Terminal(
                format!("터미널을 찾을 수 없습니다: {}", terminal_id)
            ))?;

        // 포커스 시: 상태는 제목(✳) 기반으로 결정되므로 여기서 변경하지 않음

        let project_id = mt.instance.project_id.clone();
        let target_tab_index = mt.tab_index;
        let hwnd = mt.hwnd;
        let is_attached = mt.attached;
        let runtime_id = mt.runtime_id.clone();
        let instance = mt.instance.clone();
        // mt borrow ends here (NLL)

        // 같은 프로젝트의 살아있는 탭 중 현재 탭보다 먼저 생성된 것의 수 = 유효 탭 인덱스
        let effective_index = terminals.values()
            .filter(|other| other.instance.project_id == project_id && other.tab_index < target_tab_index)
            .count();

        drop(terminals);

        if is_attached {
            // 어태치된 터미널: HWND로 직접 포커스 + UIA로 탭 선택
            if let Some(hwnd_val) = hwnd {
                let h = hwnd_val as HWND;
                // SAFETY: IsWindow is safe to call with any HWND; returns FALSE for invalid handles.
                if unsafe { IsWindow(h) } != 0 {
                    let _ = focus_window(h);
                    if let Some(ref rid) = runtime_id {
                        select_tab_via_uia(hwnd_val, rid);
                    }
                }
            }
        } else {
            // Termi가 실행한 터미널: wt.exe 명령으로 탭 전환
            let window_name = format!("termi_{}", project_id);
            let _ = std::process::Command::new(ALLOWED_TERMINAL_EXE)
                .args(["-w", &window_name, "focus-tab", "-t", &effective_index.to_string()])
                .spawn();

            // HWND로도 창을 포그라운드로 이동 (백업)
            if let Some(hwnd_val) = hwnd {
                let h = hwnd_val as HWND;
                // SAFETY: IsWindow is safe to call with any HWND; returns FALSE for invalid handles.
                if unsafe { IsWindow(h) } != 0 {
                    let _ = focus_window(h);
                }
            }
        }

        Ok(instance)
    }

    /// 터미널을 목록에서 제거하고 terminal-closed 이벤트를 발행한다.
    /// 같은 HWND에 탭이 여러 개면 해당 탭만 Ctrl+W로 닫고, 1개면 WM_CLOSE로 창 전체를 닫는다.
    /// 제거 후 남은 터미널 인스턴스 목록을 반환한다.
    pub(crate) fn close(&self, terminal_id: &str, app_handle: &tauri::AppHandle) -> Result<Vec<Instance>, AppError> {
        let mut terminals = self.lock_terminals()?;

        // remove 전에 focus-tab에 필요한 정보를 먼저 수집
        let mt = terminals.get(terminal_id)
            .ok_or_else(|| AppError::Terminal(
                format!("터미널을 찾을 수 없습니다: {}", terminal_id)
            ))?;
        let project_id = mt.instance.project_id.clone();
        let target_tab_index = mt.tab_index;
        let hwnd_opt = mt.hwnd;
        let is_attached = mt.attached;
        let runtime_id = mt.runtime_id.clone();
        // 같은 프로젝트의 살아있는 탭 중 현재 탭보다 먼저 생성된 것의 수 = 유효 탭 인덱스
        let effective_index = terminals.values()
            .filter(|other| other.instance.project_id == project_id && other.tab_index < target_tab_index)
            .count();

        let mut removed = terminals.remove(terminal_id);

        if let Some(hwnd_val) = hwnd_opt {
            let hwnd = hwnd_val as HWND;
            // SAFETY: IsWindow is safe to call with any HWND; returns FALSE for invalid handles.
            if unsafe { IsWindow(hwnd) } != 0 {
                // 같은 HWND를 쓰는 다른 Termi 관리 탭이 남아있는지 확인
                let siblings_in_same_hwnd = terminals.values()
                    .any(|mt| mt.hwnd == Some(hwnd_val));

                // UIA로 실제 Windows Terminal 탭 수 확인
                let tab_count = count_tabs_via_uia(hwnd_val);

                if tab_count == 1 && !siblings_in_same_hwnd {
                    // SAFETY: hwnd confirmed valid by IsWindow check above; PostMessageW with WM_CLOSE is a safe window operation.
                    unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0); }
                } else {
                    // 탭 닫기 후 Windows Terminal이 남은 탭들의 RuntimeId를 재할당하므로
                    // 같은 HWND의 나머지 터미널들의 RuntimeId를 초기화
                    for mt in terminals.values_mut() {
                        if mt.hwnd == Some(hwnd_val) {
                            mt.runtime_id = None;
                        }
                    }

                    // 탭 선택 + Ctrl+Shift+W 시퀀스를 별도 태스크에서 실행
                    let window_name = format!("termi_{}", project_id);
                    let hwnd_send = hwnd as usize;
                    let is_attached_for_close = is_attached;
                    let runtime_id_for_close = runtime_id.clone();
                    tauri::async_runtime::spawn(async move {
                        let hwnd = hwnd_send as HWND;

                        if is_attached_for_close {
                            // 어태치 터미널: UIA로 탭 선택
                            let _ = focus_window(hwnd);
                            if let Some(ref rid) = runtime_id_for_close {
                                select_tab_via_uia(hwnd_send as isize, rid);
                            }
                        } else {
                            // Termi 터미널: wt.exe 명령으로 탭 전환
                            let _ = std::process::Command::new(ALLOWED_TERMINAL_EXE)
                                .args(["-w", &window_name, "focus-tab", "-t", &effective_index.to_string()])
                                .spawn();
                            let _ = focus_window(hwnd);
                        }

                        tokio::time::sleep(std::time::Duration::from_millis(TAB_FOCUS_DELAY_MS)).await;
                        send_ctrl_shift_w();
                    });
                }
            } else {
                // HWND가 유효하지 않음 — Child 프로세스 강제 종료 시도
                if let Some(ref mut mt) = removed {
                    if let Some(ref mut child) = mt.child {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
        } else {
            // HWND 없음 — Child 프로세스 강제 종료 시도
            if let Some(ref mut mt) = removed {
                if let Some(ref mut child) = mt.child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }

        drop(terminals); // unlock before emit
        self.stop_reminder_for(terminal_id);
        let _ = app_handle.emit(EVENT_TERMINAL_CLOSED, terminal_id);
        self.get_all()
    }

    /// 터미널의 이름을 변경한다.
    pub(crate) fn rename(&self, terminal_id: &str, new_name: &str) -> Result<Instance, AppError> {
        // new_name 검증: 최대 255자, 영숫자·공백·'-'·'_'·'.' 만 허용
        if new_name.len() > MAX_NAME_LENGTH {
            return Err(AppError::Validation("new_name이 너무 깁니다 (최대 255자)".to_string()));
        }
        if !new_name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.')) {
            return Err(AppError::Validation(
                "new_name에 허용되지 않는 문자가 포함되어 있습니다".to_string()
            ));
        }

        let mut terminals = self.lock_terminals()?;
        let mt = terminals.get_mut(terminal_id)
            .ok_or_else(|| AppError::Terminal(
                format!("터미널을 찾을 수 없습니다: {}", terminal_id)
            ))?;
        mt.instance.terminal_name = new_name.to_string();
        Ok(mt.instance.clone())
    }

    /// 터미널의 알림 활성화 상태를 토글한다.
    pub(crate) fn toggle_notification(&self, terminal_id: &str) -> Result<Instance, AppError> {
        let mut terminals = self.lock_terminals()?;
        let mt = terminals.get_mut(terminal_id)
            .ok_or_else(|| AppError::Terminal(
                format!("터미널을 찾을 수 없습니다: {}", terminal_id)
            ))?;
        mt.instance.notification_enabled = !mt.instance.notification_enabled;
        Ok(mt.instance.clone())
    }

    /// 연결된 리마인더를 정지한다.
    /// close() 및 모니터링 루프의 종료 감지에서 공통으로 사용.
    fn stop_reminder_for(&self, terminal_id: &str) {
        if let Some(ref reminder) = self.reminder {
            if let Ok(mut r) = reminder.lock() {
                r.stop_reminder(terminal_id);
            }
        }
    }

    /// 종료된 터미널 목록에 대해 리마인더를 일괄 정지한다.
    /// 모니터링 루프에서 terminals lock 해제 후 호출.
    #[cfg(test)]
    fn stop_reminders_for_closed(&self, closed_ids: &[String]) {
        for id in closed_ids {
            self.stop_reminder_for(id);
        }
    }

    #[cfg(test)]
    fn close_and_stop_reminder(&self, terminal_id: &str) {
        {
            let Ok(mut terminals) = self.lock_terminals() else { return; };
            terminals.remove(terminal_id);
        }
        self.stop_reminder_for(terminal_id);
    }

    #[cfg(test)]
    pub(crate) fn register_terminal_for_test(
        &self,
        instance: Instance,
    ) -> Result<(), AppError> {
        let mut terminals = self.lock_terminals()?;
        let (_, tab_index) = next_terminal_number_and_tab(&terminals, &instance.project_id);
        terminals.insert(instance.id.clone(), ManagedTerminal {
            instance,
            hwnd: None,
            tab_index,
            runtime_id: None,
            title_tracking: false,
            pending_active_since: None,
            child: None,
            hook_session_id: None,
            tab_renamed: false,
            attached: false,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::models::terminal::{Activity, Instance, Status};
    use crate::services::debug_log::DebugLog;
    use crate::services::reminder::Reminder;
    use super::Manager;

    fn make_instance(id: &str, project_id: &str, name: &str) -> Instance {
        Instance {
            id: id.to_string(),
            project_id: project_id.to_string(),
            project_name: name.to_string(),
            project_path: format!("C:\\projects\\{}", name),
            terminal_name: format!("터미널 1"),
            status: Status::Running,
            launched_at: "12345Z".to_string(),
            activity: Activity::Active,
            notification_enabled: true,
            monitored: false,
            attached: false,
            last_idle_at: None,
        }
    }

    fn test_pm() -> Manager {
        Manager::new(Arc::new(DebugLog::new_for_test()))
    }

    /// 리마인더가 연결된 Manager를 생성한다.
    fn test_pm_with_reminder() -> (Manager, Arc<Mutex<Reminder>>) {
        let mut pm = Manager::new(Arc::new(DebugLog::new_for_test()));
        let reminder = Arc::new(Mutex::new(Reminder::new_for_test()));
        pm.set_reminder(Arc::clone(&reminder));
        (pm, reminder)
    }

    #[test]
    fn new_process_manager_has_empty_terminal_list() {
        let pm = test_pm();
        let terminals = pm.get_all().unwrap();
        assert!(terminals.is_empty());
    }

    #[test]
    fn register_terminal_adds_to_list() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test-project")).unwrap();

        let all = pm.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "t1");
        assert_eq!(all[0].project_name, "test-project");
        assert_eq!(all[0].status, Status::Running);
    }

    #[test]
    fn get_all_returns_multiple_terminals() {
        let pm = test_pm();
        for i in 0..3 {
            pm.register_terminal_for_test(make_instance(
                &format!("t{}", i),
                &format!("p{}", i),
                &format!("project-{}", i),
            )).unwrap();
        }
        assert_eq!(pm.get_all().unwrap().len(), 3);
    }

    #[test]
    fn register_terminal_with_same_id_overwrites() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "first")).unwrap();
        pm.register_terminal_for_test(make_instance("t1", "p2", "second")).unwrap();

        let all = pm.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].project_name, "second");
    }

    #[test]
    fn focus_returns_error_for_unknown_terminal() {
        let pm = test_pm();
        assert!(pm.focus("nonexistent").is_err());
    }

    #[test]
    fn remove_terminal_from_map_directly() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test")).unwrap();
        pm.register_terminal_for_test(make_instance("t2", "p2", "test2")).unwrap();

        {
            let mut terminals = pm.terminals.lock().unwrap();
            terminals.remove("t1");
        }

        let all = pm.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "t2");
    }

    #[test]
    fn removing_one_terminal_does_not_affect_others() {
        let pm = test_pm();
        for i in 0..5 {
            pm.register_terminal_for_test(make_instance(
                &format!("t{}", i),
                &format!("p{}", i),
                &format!("project-{}", i),
            )).unwrap();
        }

        {
            let mut terminals = pm.terminals.lock().unwrap();
            terminals.remove("t2");
        }

        let all = pm.get_all().unwrap();
        assert_eq!(all.len(), 4);
        assert!(all.iter().all(|t| t.id != "t2"));
    }

    #[test]
    fn toggle_notification_enabled() {
        let pm = test_pm();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test")).unwrap();

        // 기본값은 true
        let all = pm.get_all().unwrap();
        assert!(all[0].notification_enabled);

        // 토글 후 false
        pm.toggle_notification("t1").unwrap();
        let all = pm.get_all().unwrap();
        assert!(!all[0].notification_enabled);

        // 다시 토글 후 true
        pm.toggle_notification("t1").unwrap();
        let all = pm.get_all().unwrap();
        assert!(all[0].notification_enabled);
    }

    #[test]
    fn toggle_notification_returns_error_for_unknown_terminal() {
        let pm = test_pm();
        assert!(pm.toggle_notification("nonexistent").is_err());
    }

    #[test]
    fn close_stops_reminder_for_closed_terminal() {
        let (pm, reminder) = test_pm_with_reminder();
        pm.register_terminal_for_test(make_instance("t1", "p1", "test-project")).unwrap();

        // 리마인더를 활성 상태로 설정
        {
            let mut r = reminder.lock().unwrap();
            r.insert_fake_active("t1");
            assert!(r.has_active("t1"), "precondition: reminder should be active");
        }

        // close_and_stop_reminder: 터미널 제거 + 리마인더 정지를 원자적으로 수행
        pm.close_and_stop_reminder("t1");

        // 검증: close 후 리마인더가 정지되어야 함
        let r = reminder.lock().unwrap();
        assert!(!r.has_active("t1"), "reminder should be stopped after close");
    }

    #[test]
    fn close_stops_only_closed_terminal_reminder_others_remain() {
        let (pm, reminder) = test_pm_with_reminder();
        pm.register_terminal_for_test(make_instance("t1", "p1", "project-a")).unwrap();
        pm.register_terminal_for_test(make_instance("t2", "p2", "project-b")).unwrap();

        // 두 터미널 모두 리마인더 활성
        {
            let mut r = reminder.lock().unwrap();
            r.insert_fake_active("t1");
            r.insert_fake_active("t2");
            assert_eq!(r.active_count(), 2);
        }

        // t1만 닫기
        pm.close_and_stop_reminder("t1");

        // 검증: t1 리마인더만 정지, t2는 계속 동작
        let r = reminder.lock().unwrap();
        assert!(!r.has_active("t1"), "closed terminal's reminder should be stopped");
        assert!(r.has_active("t2"), "other terminal's reminder should remain active");
    }

    #[test]
    fn monitoring_loop_batch_close_stops_all_closed_reminders() {
        // 모니터링 루프가 여러 터미널의 종료를 감지했을 때,
        // 각 종료된 터미널의 리마인더가 정지되는지 검증
        let (pm, reminder) = test_pm_with_reminder();
        pm.register_terminal_for_test(make_instance("t1", "p1", "project-a")).unwrap();
        pm.register_terminal_for_test(make_instance("t2", "p2", "project-b")).unwrap();
        pm.register_terminal_for_test(make_instance("t3", "p3", "project-c")).unwrap();

        // 세 터미널 모두 리마인더 활성
        {
            let mut r = reminder.lock().unwrap();
            r.insert_fake_active("t1");
            r.insert_fake_active("t2");
            r.insert_fake_active("t3");
            assert_eq!(r.active_count(), 3);
        }

        // 모니터링 루프가 감지한 종료 터미널 목록
        let closed_ids = vec!["t1".to_string(), "t3".to_string()];

        // 모니터링 루프의 동작 시뮬레이션:
        // Phase 2: terminals lock 안에서 제거
        {
            let mut guard = pm.terminals.lock().unwrap();
            for id in &closed_ids {
                guard.remove(id);
            }
        }
        // Phase 3: lock 해제 후 stop_reminder 호출
        pm.stop_reminders_for_closed(&closed_ids);

        // 검증: t1, t3 리마인더 정지, t2는 유지
        let r = reminder.lock().unwrap();
        assert!(!r.has_active("t1"), "closed t1 reminder should be stopped");
        assert!(r.has_active("t2"), "surviving t2 reminder should remain active");
        assert!(!r.has_active("t3"), "closed t3 reminder should be stopped");
    }
}
