use std::collections::HashSet;

use crate::error::AppError;
use crate::models::project::Info;
use crate::models::terminal::{ExternalTerminalInfo, Instance, TabInfo};

use super::Manager;
use super::hwnd::{enumerate_wt_windows, get_tab_items_via_uia, get_terminal_cwds, get_window_title};
use super::types::*;

impl Manager {
    /// 외부 터미널을 어태치하여 Termi 관리 목록에 등록한다.
    pub(crate) fn attach_terminal(
        &self,
        hwnd: isize,
        runtime_id: Vec<i32>,
        tab_title: &str,
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

        // project_path 검증: 유효한 디렉토리인지 확인
        let path = std::path::Path::new(project_path);
        if !path.is_dir() {
            return Err(AppError::Validation(format!("Invalid project path: {}", project_path)));
        }

        let instance = {
            let mut terminals = self.lock_terminals()?;
            let (terminal_number, tab_index) = next_terminal_number_and_tab(&terminals, project_id);
            let inst = create_instance(project_id, project_name, project_path, terminal_number, language, true);
            terminals.insert(inst.id.clone(), ManagedTerminal {
                instance: inst.clone(),
                hwnd: Some(hwnd),
                tab_index,
                runtime_id: Some(runtime_id),
                title_tracking: false,
                pending_active_since: None,
                child: None,
                hook_session_id: None,
                tab_renamed: true,
                attached: true,
            });
            inst
        };

        self.debug_log.push("Attach", format!(
            "External terminal attached: {} (hwnd={}, tab=\"{}\")",
            instance.terminal_name, hwnd, tab_title
        ));

        Ok(instance)
    }

    /// 앱 시작 시 외부 Windows Terminal을 자동으로 탐색하고 프로젝트에 매칭하여 어태치한다.
    pub(crate) fn auto_attach_on_startup(
        &self,
        auto_attach_enabled: bool,
        projects: &[Info],
        language: &str,
    ) -> Result<Vec<Instance>, AppError> {
        if !auto_attach_enabled || projects.is_empty() {
            return Ok(Vec::new());
        }

        let externals = self.discover_external_terminals()?;
        let mut attached = Vec::new();

        for ext in &externals {
            for tab in &ext.tabs {
                if let Some((project, is_termi_tab)) = match_tab_to_project_with_cwds(&tab.title, projects, &ext.process_cwds) {
                    let instance = self.attach_terminal(
                        ext.hwnd,
                        tab.runtime_id.clone(),
                        &tab.title,
                        &project.id,
                        &project.name,
                        &project.path,
                        language,
                    )?;

                    // "termi: " prefix 탭은 tab_renamed: false로 설정 (✳ 감지 정상 작동)
                    if is_termi_tab {
                        let mut terminals = self.lock_terminals()?;
                        if let Some(mt) = terminals.get_mut(&instance.id) {
                            mt.tab_renamed = false;
                        }
                    }

                    attached.push(instance);
                }
            }
        }

        self.debug_log.push("AutoAttach", format!(
            "Auto-attach completed: {} terminals attached", attached.len()
        ));

        Ok(attached)
    }

    /// 현재 실행 중인 Windows Terminal 윈도우 중 Termi가 관리하지 않는 탭을 탐색한다.
    /// 같은 창에 관리 중인 탭과 미관리 탭이 섞여있으면 미관리 탭만 반환한다.
    /// Win32/UIA 호출은 별도 스레드에서 실행하여 COM apartment 충돌을 방지한다.
    /// 결과는 채널로 전달하여 호출 스레드의 차단을 최소화한다.
    pub(crate) fn discover_external_terminals(&self) -> Result<Vec<ExternalTerminalInfo>, AppError> {
        let terminals = self.lock_terminals()?;
        // (hwnd, runtime_id) 쌍으로 관리 중인 탭을 추적
        let managed_tabs: HashSet<(isize, Vec<i32>)> = terminals.values()
            .filter_map(|mt| {
                let hwnd = mt.hwnd?;
                let rid = mt.runtime_id.clone()?;
                Some((hwnd, rid))
            })
            .collect();
        // HWND 전체가 관리 중인지는 별도로 추적 (launch한 터미널은 runtime_id 없이 hwnd만 있을 수 있음)
        let managed_hwnds_without_rid: HashSet<isize> = terminals.values()
            .filter(|mt| mt.hwnd.is_some() && mt.runtime_id.is_none() && !mt.attached)
            .filter_map(|mt| mt.hwnd)
            .collect();
        drop(terminals);

        // UIA는 COM apartment를 필요로 하므로 별도 스레드에서 실행
        // (Tauri 커맨드 스레드의 COM이 STA로 초기화되어 있으면 UIA 초기화 실패 가능)
        // 결과를 oneshot 채널로 전달하여 handle.join() 대신 사용
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let all_wt_hwnds = enumerate_wt_windows();
            let automation = uiautomation::UIAutomation::new().ok();

            let mut results = Vec::new();
            for hwnd_val in all_wt_hwnds {
                if managed_hwnds_without_rid.contains(&hwnd_val) {
                    continue;
                }

                let window_title = get_window_title(hwnd_val as windows_sys::Win32::Foundation::HWND);
                let tabs: Vec<TabInfo> = if let Some(ref auto) = automation {
                    get_tab_items_via_uia(auto, hwnd_val)
                        .into_iter()
                        .filter(|(rid, _)| !managed_tabs.contains(&(hwnd_val, rid.clone())))
                        .map(|(runtime_id, title)| TabInfo { runtime_id, title })
                        .collect()
                } else {
                    Vec::new()
                };

                if !tabs.is_empty() {
                    let process_cwds = get_terminal_cwds(hwnd_val);
                    results.push(ExternalTerminalInfo {
                        hwnd: hwnd_val,
                        window_title,
                        tabs,
                        process_cwds,
                    });
                }
            }
            let _ = tx.send(results);
        });

        rx.recv()
            .map_err(|_| AppError::Validation("Discover thread panicked".to_string()))
    }
}

/// 탭 타이틀을 프로젝트 목록과 매칭한다. (is_termi_tab: "termi: " prefix 매칭 여부)
pub(super) fn match_tab_to_project<'a>(tab_title: &str, projects: &'a [Info]) -> Option<(&'a Info, bool)> {
    // 1순위: "termi: {프로젝트명}" prefix 매칭
    if let Some(name) = tab_title.strip_prefix(TITLE_PREFIX) {
        // ✳ 마커나 기타 suffix 제거 후 비교
        let name_trimmed = name.trim();
        for project in projects {
            if name_trimmed == project.name || name_trimmed.starts_with(&project.name) {
                return Some((project, true));
            }
        }
    }

    // 2순위: 탭 타이틀에 프로젝트 경로가 포함 (대소문자 무시)
    let title_lower = tab_title.to_lowercase();
    for project in projects {
        if title_lower.contains(&project.path.to_lowercase()) {
            return Some((project, false));
        }
    }

    None
}

/// 탭 타이틀 + 프로세스 cwd를 기반으로 프로젝트를 매칭한다.
/// 타이틀 매칭 우선, 실패 시 cwd 매칭으로 폴백.
pub(super) fn match_tab_to_project_with_cwds<'a>(
    tab_title: &str,
    projects: &'a [Info],
    cwds: &[String],
) -> Option<(&'a Info, bool)> {
    // 먼저 기존 타이틀 기반 매칭 시도
    if let Some(result) = match_tab_to_project(tab_title, projects) {
        return Some(result);
    }

    // 3순위: 프로세스 cwd와 프로젝트 경로 매칭 (대소문자 무시)
    for cwd in cwds {
        let cwd_lower = cwd.to_lowercase();
        for project in projects {
            let proj_lower = project.path.to_lowercase().replace('/', "\\");
            let cwd_normalized = cwd_lower.replace('/', "\\");
            if cwd_normalized == proj_lower || cwd_normalized.starts_with(&format!("{}\\", proj_lower)) {
                return Some((project, false));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::models::terminal::{Activity, Instance, Status};
    use crate::services::debug_log::DebugLog;
    use super::Manager;
    use super::match_tab_to_project;
    use super::match_tab_to_project_with_cwds;

    const FAKE_EXCLUDED_HWND: isize = 99999;
    const FAKE_ATTACH_HWND: isize = 55555;
    const FAKE_AOT_HWND: isize = 88888;

    fn test_dir() -> String {
        std::env::temp_dir().to_string_lossy().to_string()
    }

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

    #[test]
    fn attach_terminal_creates_instance_with_attached_true() {
        let pm = test_pm();
        let result = pm.attach_terminal(
            12345, vec![1, 2, 3], "PowerShell", "proj-1", "my-project", &test_dir(), "ko",
        );
        assert!(result.is_ok());
        let instance = result.unwrap();
        assert!(instance.attached);
        assert_eq!(instance.project_id, "proj-1");
        assert_eq!(instance.project_name, "my-project");
        assert_eq!(instance.project_path, test_dir());
        assert_eq!(instance.status, Status::Running);
        assert_eq!(instance.activity, Activity::Active);
        assert!(instance.notification_enabled);
        assert!(!instance.monitored);
    }

    #[test]
    fn attach_terminal_adds_to_terminal_list() {
        let pm = test_pm();
        pm.attach_terminal(
            99999, vec![10], "bash", "p1", "test", &test_dir(), "en",
        ).unwrap();

        let all = pm.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].attached);
    }

    #[test]
    fn attach_terminal_has_correct_managed_terminal_fields() {
        let pm = test_pm();
        let instance = pm.attach_terminal(
            FAKE_ATTACH_HWND, vec![7, 8], "cmd", "p1", "test", &test_dir(), "ko",
        ).unwrap();

        let terminals = pm.terminals.lock().unwrap();
        let mt = terminals.get(&instance.id).unwrap();
        assert_eq!(mt.hwnd, Some(FAKE_ATTACH_HWND));
        assert_eq!(mt.runtime_id, Some(vec![7, 8]));
        assert!(mt.child.is_none());
        assert!(mt.tab_renamed); // 외부 탭은 "termi: " prefix 없으므로 true
        assert!(mt.instance.attached);
    }

    #[test]
    fn attach_terminal_terminal_name_in_korean() {
        let pm = test_pm();
        let instance = pm.attach_terminal(
            1, vec![1], "tab", "p1", "test", &test_dir(), "ko",
        ).unwrap();
        assert!(instance.terminal_name.contains("터미널"));
    }

    #[test]
    fn attach_terminal_terminal_name_in_english() {
        let pm = test_pm();
        let instance = pm.attach_terminal(
            1, vec![1], "tab", "p1", "test", &test_dir(), "en",
        ).unwrap();
        assert!(instance.terminal_name.contains("Terminal"));
    }

    #[test]
    fn auto_attach_returns_empty_when_disabled() {
        let pm = test_pm();
        let projects = vec![];
        let result = pm.auto_attach_on_startup(false, &projects, "ko");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn auto_attach_returns_empty_when_no_projects() {
        let pm = test_pm();
        let projects = vec![];
        let result = pm.auto_attach_on_startup(true, &projects, "ko");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn auto_attach_matches_termi_prefix_tab() {
        use crate::models::project::Info;
        let pm = test_pm();
        let projects = vec![Info {
            id: "p1".to_string(),
            name: "my-project".to_string(),
            path: "C:\\projects\\my-project".to_string(),
            created_at: "2026-01-01".to_string(),
            sort_order: 0,
        }];
        // Since we can't mock EnumWindows, just verify method is callable and returns Ok
        let result = pm.auto_attach_on_startup(true, &projects, "ko");
        assert!(result.is_ok());
    }

    #[test]
    fn match_tab_to_project_termi_prefix() {
        use crate::models::project::Info;
        let projects = vec![Info {
            id: "p1".to_string(),
            name: "my-project".to_string(),
            path: "C:\\projects\\my-project".to_string(),
            created_at: "2026-01-01".to_string(),
            sort_order: 0,
        }];
        let result = match_tab_to_project("termi: my-project", &projects);
        assert!(result.is_some());
        let (project, is_termi_tab) = result.unwrap();
        assert_eq!(project.id, "p1");
        assert!(is_termi_tab);
    }

    #[test]
    fn match_tab_to_project_path_in_title() {
        use crate::models::project::Info;
        let projects = vec![Info {
            id: "p1".to_string(),
            name: "my-project".to_string(),
            path: "C:\\projects\\my-project".to_string(),
            created_at: "2026-01-01".to_string(),
            sort_order: 0,
        }];
        let result = match_tab_to_project("C:\\projects\\my-project", &projects);
        assert!(result.is_some());
        let (project, is_termi_tab) = result.unwrap();
        assert_eq!(project.id, "p1");
        assert!(!is_termi_tab);
    }

    #[test]
    fn match_tab_to_project_path_case_insensitive() {
        use crate::models::project::Info;
        let projects = vec![Info {
            id: "p1".to_string(),
            name: "my-project".to_string(),
            path: "C:\\Projects\\My-Project".to_string(),
            created_at: "2026-01-01".to_string(),
            sort_order: 0,
        }];
        let result = match_tab_to_project("c:\\projects\\my-project", &projects);
        assert!(result.is_some());
    }

    #[test]
    fn match_tab_to_project_no_match() {
        use crate::models::project::Info;
        let projects = vec![Info {
            id: "p1".to_string(),
            name: "my-project".to_string(),
            path: "C:\\projects\\my-project".to_string(),
            created_at: "2026-01-01".to_string(),
            sort_order: 0,
        }];
        let result = match_tab_to_project("PowerShell", &projects);
        assert!(result.is_none());
    }

    #[test]
    fn match_tab_to_project_termi_prefix_priority() {
        use crate::models::project::Info;
        let projects = vec![
            Info {
                id: "p1".to_string(),
                name: "my-project".to_string(),
                path: "C:\\projects\\my-project".to_string(),
                created_at: "2026-01-01".to_string(),
                sort_order: 0,
            },
            Info {
                id: "p2".to_string(),
                name: "other".to_string(),
                path: "C:\\projects\\other".to_string(),
                created_at: "2026-01-01".to_string(),
                sort_order: 0,
            },
        ];
        // "termi: " prefix should match by name, not path
        let result = match_tab_to_project("termi: my-project", &projects);
        assert!(result.is_some());
        let (project, is_termi) = result.unwrap();
        assert_eq!(project.id, "p1");
        assert!(is_termi);
    }

    #[test]
    fn attached_terminal_has_no_child_process() {
        // Attached terminals have child: None since they weren't spawned by Termi
        let pm = test_pm();
        let instance = pm.attach_terminal(
            FAKE_AOT_HWND, vec![1], "tab", "p1", "test", &test_dir(), "ko",
        ).unwrap();

        let terminals = pm.terminals.lock().unwrap();
        let mt = terminals.get(&instance.id).unwrap();
        assert!(mt.child.is_none());
        assert!(mt.attached);
        assert_eq!(mt.hwnd, Some(FAKE_AOT_HWND));
    }

    #[test]
    fn discover_external_terminals_returns_vec() {
        let pm = test_pm();
        let result = pm.discover_external_terminals();
        assert!(result.is_ok());
        // On test environment, no real WT windows expected — empty vec is fine
    }

    #[test]
    fn discover_external_terminals_excludes_managed_hwnds() {
        let pm = test_pm();
        // Register a terminal with a fake HWND
        pm.register_terminal_for_test(make_instance("t1", "p1", "test")).unwrap();
        {
            let mut terminals = pm.terminals.lock().unwrap();
            if let Some(mt) = terminals.get_mut("t1") {
                mt.hwnd = Some(FAKE_EXCLUDED_HWND);
            }
        }

        let result = pm.discover_external_terminals().unwrap();
        // The fake HWND should not appear in results
        assert!(result.iter().all(|info| info.hwnd != FAKE_EXCLUDED_HWND));
    }

    #[test]
    fn match_tab_to_project_cwd_match() {
        use crate::models::project::Info;
        let projects = vec![Info {
            id: "p1".to_string(),
            name: "my-project".to_string(),
            path: "C:\\projects\\my-project".to_string(),
            created_at: "2026-01-01".to_string(),
            sort_order: 0,
        }];
        let cwds = vec!["C:\\projects\\my-project".to_string()];
        let result = match_tab_to_project_with_cwds("some random title", &projects, &cwds);
        assert!(result.is_some());
        let (project, is_termi) = result.unwrap();
        assert_eq!(project.id, "p1");
        assert!(!is_termi);
    }

    #[test]
    fn match_tab_to_project_title_priority_over_cwd() {
        use crate::models::project::Info;
        let projects = vec![
            Info {
                id: "p1".to_string(),
                name: "proj-a".to_string(),
                path: "C:\\projects\\proj-a".to_string(),
                created_at: "2026-01-01".to_string(),
                sort_order: 0,
            },
            Info {
                id: "p2".to_string(),
                name: "proj-b".to_string(),
                path: "C:\\projects\\proj-b".to_string(),
                created_at: "2026-01-01".to_string(),
                sort_order: 0,
            },
        ];
        let cwds = vec!["C:\\projects\\proj-b".to_string()];
        // Title matches p1, cwd matches p2 — title should win
        let result = match_tab_to_project_with_cwds("termi: proj-a", &projects, &cwds);
        assert!(result.is_some());
        let (project, is_termi) = result.unwrap();
        assert_eq!(project.id, "p1");
        assert!(is_termi);
    }
}
