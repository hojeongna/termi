pub(crate) mod debug_log;
pub(crate) mod event_watcher;
pub(crate) mod process_manager;
pub(crate) mod notifier;
pub(crate) mod reminder;

use std::sync::{Arc, Mutex};

use tauri::Emitter;

use crate::events;
use crate::models::terminal::{Activity, StatusChangedPayload};
pub(crate) use process_manager::types::TransitionTuple;

/// 활동 전환 목록을 처리한다: 상태 변경 이벤트 emit, 알림 전송, 리마인더 시작/정지.
/// monitoring.rs (✳ 탭 제목 기반 감지)와 lib.rs (Hook 기반 감지) 양쪽에서 공통으로 호출.
pub(crate) fn dispatch_transitions(
    app_handle: &tauri::AppHandle,
    notifier: &Arc<Mutex<notifier::Notifier>>,
    reminder: &Arc<Mutex<reminder::Reminder>>,
    transitions: &[TransitionTuple],
) {
    for (terminal_id, project_path, project_name, terminal_name, new_activity, notification_enabled, monitored, last_idle_at) in transitions {
        // 상태 변경 이벤트 emit
        if let Err(e) = app_handle.emit(events::TERMINAL_STATUS_CHANGED, &StatusChangedPayload {
            project_path: project_path.clone(),
            status: new_activity.clone(),
            terminal_id: terminal_id.clone(),
            monitored: *monitored,
            last_idle_at: last_idle_at.clone(),
        }) {
            eprintln!("Failed to emit terminal-status-changed: {e}");
        }

        match new_activity {
            Activity::Idle => {
                // notification_enabled가 false면 알림/리마인더 건너뛰기
                if !notification_enabled {
                    continue;
                }
                // Idle 전환: 알림 + 리마인더 시작
                let sent_ok = if let Ok(mut n) = notifier.lock() {
                    match n.send_status_notification(app_handle, terminal_id, project_name, terminal_name, &Activity::Idle) {
                        Ok(()) => true,
                        Err(e) => {
                            eprintln!("알림 발생 실패 (무시됨): {e}");
                            false
                        }
                    }
                } else {
                    false
                };

                if sent_ok {
                    if let Ok(mut r) = reminder.lock() {
                        r.start_reminder(terminal_id, project_name, terminal_name, notifier);
                    }
                }
            }
            Activity::Active => {
                // Active 복귀: 리마인더 정지 (알림 없음)
                if let Ok(mut r) = reminder.lock() {
                    r.stop_reminder(terminal_id);
                }
            }
        }
    }
}
