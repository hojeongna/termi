use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use tauri::Emitter;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::models::terminal::{Activity, Status};
use crate::services::notifier;
use crate::services::reminder::Reminder;

use super::Manager;
use super::hwnd::{get_tab_items_via_uia, get_window_title};
use super::types::*;

/// 순서 계산에 필요한 최소 터미널 수.
/// HWND당 관리 터미널이 이 수 미만이면 순서가 의미 없으므로 결과에서 제외한다.
const MIN_TERMINALS_FOR_ORDER: usize = 2;

/// UIA 탭 순서(좌→우)와 관리 터미널을 RuntimeId로 매칭하여
/// HWND별 터미널 ID 순서를 계산한다.
/// 2개 이상의 관리 터미널이 매칭된 HWND만 결과에 포함한다.
pub(super) fn compute_terminal_order(
    uia_data: &HashMap<isize, Vec<(Vec<i32>, String)>>,
    terminals: &HashMap<String, ManagedTerminal>,
) -> HashMap<isize, Vec<String>> {
    // RuntimeId → terminal ID 역방향 조회 맵 구축 (O(1) lookup)
    let rid_to_id: HashMap<&Vec<i32>, &String> = terminals.iter()
        .filter_map(|(id, mt)| mt.runtime_id.as_ref().map(|rid| (rid, id)))
        .collect();

    let mut result: HashMap<isize, Vec<String>> = HashMap::new();
    for (&hwnd, uia_tabs) in uia_data {
        let mut ids: Vec<String> = Vec::new();
        for (rid, _title) in uia_tabs {
            // RuntimeId로 관리 터미널 매칭
            if let Some(id) = rid_to_id.get(rid) {
                ids.push((*id).clone());
            }
        }
        if ids.len() >= MIN_TERMINALS_FOR_ORDER {
            result.insert(hwnd, ids);
        }
    }
    result
}

impl Manager {
    /// 모든 터미널의 HWND 유효성과 IO 활동을 주기적으로 확인하는 백그라운드 스레드 시작.
    /// Notifier와 Reminder에 대한 Arc를 받아 상태 전환 시 알림/리마인더 파이프라인을 실행한다.
    pub(crate) fn start_monitoring(
        &self,
        app_handle: tauri::AppHandle,
        notifier: Arc<Mutex<notifier::Notifier>>,
        reminder: Arc<Mutex<Reminder>>,
    ) {
        let terminals = Arc::clone(&self.terminals);
        let debug_log = Arc::clone(&self.debug_log);
        debug_log.push("Monitor", "Monitoring started (UIA per-tab ✳ detection)".to_string());

        tauri::async_runtime::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || {
            // UIA 초기화 (이 스레드에서만 사용)
            let automation = match uiautomation::UIAutomation::new() {
                Ok(a) => {
                    debug_log.push("UIA", "UI Automation initialized — per-tab title detection enabled".to_string());
                    Some(a)
                }
                Err(e) => {
                    debug_log.push("UIA", format!("UI Automation init failed: {e} — fallback to window title"));
                    None
                }
            };

            let mut poll_count: u64 = 0;
            let mut previous_terminal_order: HashMap<isize, Vec<String>> = HashMap::new();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(MONITOR_POLL_INTERVAL_SECS));
                poll_count += 1;

                // Phase 0: HWND 수집 (lock 최소화)
                let hwnd_set: HashSet<isize> = {
                    let guard = match terminals.lock() {
                        Ok(g) => g,
                        Err(_) => continue,
                    };
                    guard.values()
                        .filter(|mt| mt.instance.status == Status::Running)
                        .filter_map(|mt| mt.hwnd)
                        .collect()
                };

                // Phase 1: 각 HWND별 창 유효성 확인 + UIA 탭 데이터 수집 (lock 없음)
                // Vec<(RuntimeId, title)> per HWND
                let mut uia_data: HashMap<isize, Vec<(Vec<i32>, String)>> = HashMap::new();
                let mut fallback_titles: HashMap<isize, String> = HashMap::new();
                let mut dead_hwnds: HashSet<isize> = HashSet::new();

                for &hwnd_val in &hwnd_set {
                    let hwnd = hwnd_val as HWND;
                    // SAFETY: IsWindow is safe to call with any HWND; returns FALSE for invalid handles.
                    if unsafe { IsWindow(hwnd) } == 0 {
                        dead_hwnds.insert(hwnd_val);
                        continue;
                    }
                    if let Some(ref auto) = automation {
                        let tabs = get_tab_items_via_uia(auto, hwnd_val);
                        if !tabs.is_empty() {
                            uia_data.insert(hwnd_val, tabs);
                            continue;
                        }
                    }
                    // UIA 실패 시 폴백: 창 제목
                    fallback_titles.insert(hwnd_val, get_window_title(hwnd));
                }

                // Phase 2: RuntimeId 매칭 + 탭 종료 감지 + 활동 감지 (lock)
                let mut closed_ids: Vec<String> = Vec::new();
                let mut transitions: Vec<TransitionTuple> = Vec::new();
                let order_changed_payload;

                {
                    let mut guard = match terminals.lock() {
                        Ok(g) => g,
                        Err(_) => continue,
                    };

                    // 2a: HWND별 그룹 구성 + RuntimeId 기반 탭 매칭
                    let mut tab_title_for: HashMap<String, String> = HashMap::new();

                    // HWND별로 터미널 ID 그룹화
                    let mut hwnd_groups: HashMap<isize, Vec<String>> = HashMap::new();
                    for (id, mt) in guard.iter() {
                        if mt.instance.status != Status::Running { continue; }
                        if let Some(h) = mt.hwnd {
                            if dead_hwnds.contains(&h) {
                                closed_ids.push(id.clone());
                            } else {
                                hwnd_groups.entry(h).or_default().push(id.clone());
                            }
                        }
                    }

                    for (hwnd_val, ids) in &hwnd_groups {
                        if let Some(uia_tabs) = uia_data.get(hwnd_val) {
                            // UIA 데이터 있음 → RuntimeId 기반 매칭
                            let uia_rid_map: HashMap<&Vec<i32>, &String> = uia_tabs.iter()
                                .map(|(rid, title)| (rid, title))
                                .collect();

                            // RuntimeId가 있는 터미널: 직접 매칭
                            let mut unassigned_ids: Vec<String> = Vec::new();
                            for id in ids {
                                let mt = &guard[id];
                                if let Some(ref rid) = mt.runtime_id {
                                    if let Some(title) = uia_rid_map.get(rid) {
                                        tab_title_for.insert(id.clone(), (*title).clone());
                                    } else {
                                        // RuntimeId가 UIA에 없음 → 탭이 외부에서 종료됨
                                        debug_log.push("Close", format!(
                                            "{}: External tab close detected (RuntimeId lost)",
                                            mt.instance.terminal_name
                                        ));
                                        closed_ids.push(id.clone());
                                    }
                                } else {
                                    unassigned_ids.push(id.clone());
                                }
                            }

                            // RuntimeId 미할당 터미널 → 사용 가능한 UIA 탭에 순서대로 할당
                            if !unassigned_ids.is_empty() {
                                let assigned_rids: HashSet<&Vec<i32>> = ids.iter()
                                    .filter_map(|id| guard.get(id)?.runtime_id.as_ref())
                                    .collect();
                                let available: Vec<&(Vec<i32>, String)> = uia_tabs.iter()
                                    .filter(|(rid, _)| !assigned_rids.contains(rid))
                                    .collect();

                                let mut sorted: Vec<_> = unassigned_ids.iter()
                                    .map(|id| (id.clone(), guard[id].tab_index))
                                    .collect();
                                sorted.sort_by_key(|(_, ti)| *ti);

                                for (i, (id, _)) in sorted.iter().enumerate() {
                                    if let Some((rid, title)) = available.get(i) {
                                        if let Some(mt) = guard.get_mut(id) {
                                            mt.runtime_id = Some(rid.clone());
                                            debug_log.push("UIA", format!(
                                                "{}: RuntimeId assigned", mt.instance.terminal_name,
                                            ));
                                        }
                                        tab_title_for.insert(id.clone(), title.clone());
                                    }
                                }
                            }
                        } else if let Some(title) = fallback_titles.get(hwnd_val) {
                            // UIA 실패 → 폴백: 모든 터미널에 같은 창 제목
                            for id in ids {
                                tab_title_for.insert(id.clone(), title.clone());
                            }
                        }
                    }

                    // 2b: 활동 감지
                    let ids: Vec<String> = guard.keys().cloned().collect();
                    for id in &ids {
                        if closed_ids.contains(id) { continue; }

                        let mt = match guard.get_mut(id) {
                            Some(mt) => mt,
                            None => continue,
                        };
                        if mt.instance.status != Status::Running { continue; }
                        if mt.hwnd.is_none() {
                            if poll_count % LOG_THROTTLE_POLL_INTERVAL == 0 {
                                debug_log.push("HWND", format!("{}: No HWND — skipping monitoring", mt.instance.terminal_name));
                            }
                            continue;
                        }

                        let title = tab_title_for.get(id).map(|s| s.as_str()).unwrap_or("");
                        let has_marker = title.contains(TITLE_IDLE_MARKER);

                        // 탭 리네임 감지: 제목이 "termi: " prefix를 잃으면 사용자가 이름을 변경한 것
                        if !title.is_empty() && !mt.tab_renamed
                            && !title.starts_with(TITLE_PREFIX)
                            && !title.contains(TITLE_IDLE_MARKER)
                        {
                            mt.tab_renamed = true;
                            debug_log.push("Title", format!(
                                "{}: tab renamed detected (title=\"{}\")",
                                mt.instance.terminal_name, title,
                            ));
                        }

                        // ✳가 처음 나타나면 감지 대상으로 등록
                        if has_marker && !mt.title_tracking {
                            mt.title_tracking = true;
                            mt.instance.monitored = true;
                            debug_log.push("Title", format!(
                                "{}: ✳ first detected — monitoring activated",
                                mt.instance.terminal_name,
                            ));
                        }

                        if !mt.title_tracking { continue; }

                        // LOG_THROTTLE_POLL_INTERVAL 폴링마다 상태 로그
                        if poll_count % LOG_THROTTLE_POLL_INTERVAL == 0 {
                            debug_log.push("Title", format!(
                                "{}: activity={:?} title=\"{}\"",
                                mt.instance.terminal_name, mt.instance.activity, title,
                            ));
                        }

                        if has_marker {
                            mt.pending_active_since = None;
                            if mt.instance.activity != Activity::Idle {
                                debug_log.push("State", format!(
                                    "{}: {:?} → Idle (title=\"{}\")",
                                    mt.instance.terminal_name, mt.instance.activity, title,
                                ));
                                mt.instance.activity = Activity::Idle;
                                mt.instance.last_idle_at = Some(now_timestamp());
                                transitions.push((
                                    id.clone(),
                                    mt.instance.project_path.clone(),
                                    mt.instance.project_name.clone(),
                                    mt.instance.terminal_name.clone(),
                                    Activity::Idle,
                                    mt.instance.notification_enabled,
                                    mt.instance.monitored,
                                    mt.instance.last_idle_at.clone(),
                                ));
                            }
                        } else if mt.instance.activity == Activity::Idle {
                            match mt.pending_active_since {
                                None => {
                                    mt.pending_active_since = Some(std::time::Instant::now());
                                }
                                Some(since) => {
                                    if since.elapsed().as_secs() >= ACTIVE_DEBOUNCE_SECS {
                                        mt.pending_active_since = None;
                                        debug_log.push("State", format!(
                                            "{}: Idle → Active (debounce {}s, title=\"{}\")",
                                            mt.instance.terminal_name, ACTIVE_DEBOUNCE_SECS, title,
                                        ));
                                        mt.instance.activity = Activity::Active;
                                        transitions.push((
                                            id.clone(),
                                            mt.instance.project_path.clone(),
                                            mt.instance.project_name.clone(),
                                            mt.instance.terminal_name.clone(),
                                            Activity::Active,
                                            mt.instance.notification_enabled,
                                            mt.instance.monitored,
                                            mt.instance.last_idle_at.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    // 2c: 탭 순서 변경 감지
                    order_changed_payload = {
                        let current_order = compute_terminal_order(&uia_data, &guard);
                        if current_order != previous_terminal_order {
                            previous_terminal_order = current_order.clone();

                            // tab_index를 UIA 실제 위치로 갱신 (focus/close 시 올바른 탭 타겟팅)
                            for uia_tabs in uia_data.values() {
                                for (uia_pos, (rid, _title)) in uia_tabs.iter().enumerate() {
                                    if let Some(mt) = guard.values_mut()
                                        .find(|mt| mt.runtime_id.as_ref() == Some(rid))
                                    {
                                        mt.tab_index = uia_pos;
                                    }
                                }
                            }

                            if !current_order.is_empty() {
                                // 모든 HWND의 터미널 ID를 단일 Vec으로 flatten
                                let mut flat: Vec<String> = Vec::new();
                                for (_hwnd, ids) in &current_order {
                                    flat.extend(ids.iter().cloned());
                                }
                                Some(flat)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    // 종료된 터미널 제거
                    for id in &closed_ids {
                        guard.remove(id);
                    }
                }

                // Phase 3: 이벤트 emit + 리마인더 정지 (lock 해제 후)
                for id in &closed_ids {
                    if let Ok(mut r) = reminder.lock() {
                        r.stop_reminder(id);
                    }
                }
                for id in &closed_ids {
                    let _ = app_handle.emit(EVENT_TERMINAL_CLOSED, id.as_str());
                }

                // 순서 변경 이벤트 emit (lock 해제 후)
                if let Some(order_ids) = order_changed_payload {
                    debug_log.push("Order", format!("Terminal order changed: {:?}", order_ids));
                    let _ = app_handle.emit(EVENT_TERMINAL_ORDER_CHANGED, &order_ids);
                }

                crate::services::dispatch_transitions(&app_handle, &notifier, &reminder, &transitions);
            }
        }).await.ok();
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::models::terminal::{Activity, Instance, Status};
    use super::super::types::ManagedTerminal;
    use super::compute_terminal_order;

    /// 테스트용 ManagedTerminal을 생성하는 헬퍼.
    /// runtime_id를 지정할 수 있다.
    fn make_mt(id: &str, runtime_id: Option<Vec<i32>>) -> (String, ManagedTerminal) {
        (
            id.to_string(),
            ManagedTerminal {
                instance: Instance {
                    id: id.to_string(),
                    project_id: "p1".to_string(),
                    project_name: "test".to_string(),
                    project_path: "C:\\test".to_string(),
                    terminal_name: format!("Terminal {}", id),
                    status: Status::Running,
                    launched_at: "2024-01-01T00:00:00Z".to_string(),
                    activity: Activity::Active,
                    notification_enabled: true,
                    monitored: false,
                    attached: false,
                    last_idle_at: None,
                },
                hwnd: Some(1000),
                tab_index: 0,
                runtime_id,
                title_tracking: false,
                pending_active_since: None,
                child: None,
                hook_session_id: None,
                tab_renamed: false,
                attached: false,
            },
        )
    }

    #[test]
    fn empty_data_returns_empty() {
        let uia_data: HashMap<isize, Vec<(Vec<i32>, String)>> = HashMap::new();
        let terminals: HashMap<String, ManagedTerminal> = HashMap::new();

        let result = compute_terminal_order(&uia_data, &terminals);
        assert!(result.is_empty());
    }

    #[test]
    fn single_terminal_is_excluded() {
        // HWND에 관리 터미널이 1개뿐이면 순서 의미 없으므로 제외
        let hwnd: isize = 1000;
        let rid = vec![1, 2, 3];
        let mut uia_data = HashMap::new();
        uia_data.insert(hwnd, vec![(rid.clone(), "tab1".to_string())]);

        let mut terminals = HashMap::new();
        let (id, mt) = make_mt("t1", Some(rid));
        terminals.insert(id, mt);

        let result = compute_terminal_order(&uia_data, &terminals);
        assert!(result.is_empty(), "단일 터미널은 결과에 포함되지 않아야 함");
    }

    #[test]
    fn multiple_terminals_ordered_by_uia_tab_position() {
        // UIA 탭 순서: rid1(좌), rid2(중), rid3(우) → 매칭된 터미널 순서도 좌→우
        let hwnd: isize = 2000;
        let rid1 = vec![10, 20];
        let rid2 = vec![10, 30];
        let rid3 = vec![10, 40];

        let mut uia_data = HashMap::new();
        uia_data.insert(hwnd, vec![
            (rid1.clone(), "tab A".to_string()),
            (rid2.clone(), "tab B".to_string()),
            (rid3.clone(), "tab C".to_string()),
        ]);

        let mut terminals = HashMap::new();
        let (id1, mut mt1) = make_mt("t1", Some(rid1));
        mt1.hwnd = Some(hwnd);
        terminals.insert(id1, mt1);
        let (id2, mut mt2) = make_mt("t2", Some(rid2));
        mt2.hwnd = Some(hwnd);
        terminals.insert(id2, mt2);
        let (id3, mut mt3) = make_mt("t3", Some(rid3));
        mt3.hwnd = Some(hwnd);
        terminals.insert(id3, mt3);

        let result = compute_terminal_order(&uia_data, &terminals);
        assert_eq!(result.len(), 1);
        let order = result.get(&hwnd).unwrap();
        assert_eq!(order, &vec!["t1".to_string(), "t2".to_string(), "t3".to_string()]);
    }

    #[test]
    fn unmanaged_tabs_are_skipped() {
        // UIA에 3개 탭이 있지만 관리 터미널은 2개만 → 비관리 탭은 무시
        let hwnd: isize = 3000;
        let rid1 = vec![5, 1];
        let rid_unmanaged = vec![5, 2]; // 관리하지 않는 탭
        let rid3 = vec![5, 3];

        let mut uia_data = HashMap::new();
        uia_data.insert(hwnd, vec![
            (rid1.clone(), "managed tab 1".to_string()),
            (rid_unmanaged, "external tab".to_string()),
            (rid3.clone(), "managed tab 2".to_string()),
        ]);

        let mut terminals = HashMap::new();
        let (id1, mut mt1) = make_mt("t1", Some(rid1));
        mt1.hwnd = Some(hwnd);
        terminals.insert(id1, mt1);
        let (id3, mut mt3) = make_mt("t3", Some(rid3));
        mt3.hwnd = Some(hwnd);
        terminals.insert(id3, mt3);

        let result = compute_terminal_order(&uia_data, &terminals);
        assert_eq!(result.len(), 1);
        let order = result.get(&hwnd).unwrap();
        assert_eq!(order, &vec!["t1".to_string(), "t3".to_string()]);
    }

    #[test]
    fn terminals_without_runtime_id_are_skipped() {
        // RuntimeId가 없는 터미널은 매칭 불가 → 결과에서 제외
        let hwnd: isize = 4000;
        let rid1 = vec![7, 1];
        let rid2 = vec![7, 2];

        let mut uia_data = HashMap::new();
        uia_data.insert(hwnd, vec![
            (rid1.clone(), "tab 1".to_string()),
            (rid2.clone(), "tab 2".to_string()),
        ]);

        let mut terminals = HashMap::new();
        let (id1, mut mt1) = make_mt("t1", Some(rid1));
        mt1.hwnd = Some(hwnd);
        terminals.insert(id1, mt1);
        // t2는 runtime_id가 None
        let (id2, mut mt2) = make_mt("t2", None);
        mt2.hwnd = Some(hwnd);
        terminals.insert(id2, mt2);

        let result = compute_terminal_order(&uia_data, &terminals);
        // t1만 매칭되므로 1개 → 2개 미만이라 제외
        assert!(result.is_empty(), "RuntimeId 없는 터미널은 매칭 불가 → 1개만 매칭되면 제외");
    }

    #[test]
    fn multiple_hwnds_each_tracked_independently() {
        // 서로 다른 HWND에 각각 2개씩 터미널이 있는 경우
        let hwnd_a: isize = 5000;
        let hwnd_b: isize = 6000;
        let rid_a1 = vec![1, 1];
        let rid_a2 = vec![1, 2];
        let rid_b1 = vec![2, 1];
        let rid_b2 = vec![2, 2];

        let mut uia_data = HashMap::new();
        uia_data.insert(hwnd_a, vec![
            (rid_a1.clone(), "a-tab1".to_string()),
            (rid_a2.clone(), "a-tab2".to_string()),
        ]);
        uia_data.insert(hwnd_b, vec![
            (rid_b1.clone(), "b-tab1".to_string()),
            (rid_b2.clone(), "b-tab2".to_string()),
        ]);

        let mut terminals = HashMap::new();
        let (id, mut mt) = make_mt("ta1", Some(rid_a1));
        mt.hwnd = Some(hwnd_a);
        terminals.insert(id, mt);
        let (id, mut mt) = make_mt("ta2", Some(rid_a2));
        mt.hwnd = Some(hwnd_a);
        terminals.insert(id, mt);
        let (id, mut mt) = make_mt("tb1", Some(rid_b1));
        mt.hwnd = Some(hwnd_b);
        terminals.insert(id, mt);
        let (id, mut mt) = make_mt("tb2", Some(rid_b2));
        mt.hwnd = Some(hwnd_b);
        terminals.insert(id, mt);

        let result = compute_terminal_order(&uia_data, &terminals);
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&hwnd_a).unwrap(), &vec!["ta1".to_string(), "ta2".to_string()]);
        assert_eq!(result.get(&hwnd_b).unwrap(), &vec!["tb1".to_string(), "tb2".to_string()]);
    }

    #[test]
    fn uia_hwnd_with_no_matching_terminals_excluded() {
        // UIA 데이터에 HWND가 있지만 관리 터미널이 없는 경우
        let hwnd: isize = 7000;
        let rid1 = vec![9, 1];
        let rid2 = vec![9, 2];

        let mut uia_data = HashMap::new();
        uia_data.insert(hwnd, vec![
            (rid1, "tab 1".to_string()),
            (rid2, "tab 2".to_string()),
        ]);

        // 관리 터미널 없음
        let terminals: HashMap<String, ManagedTerminal> = HashMap::new();

        let result = compute_terminal_order(&uia_data, &terminals);
        assert!(result.is_empty());
    }
}
