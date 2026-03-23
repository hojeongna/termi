pub(crate) mod error;
pub(crate) mod events;
pub(crate) mod store;
pub(crate) mod models;
pub(crate) mod commands;
pub(crate) mod services;

use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager};
use crate::services::debug_log::DebugLog;
use crate::services::event_watcher::EventWatcher;
use crate::services::notifier;
use crate::services::process_manager::Manager as ProcessManager;
use crate::services::reminder::Reminder;

/// Delay before auto-attaching external terminals on startup (in milliseconds).
const AUTO_ATTACH_DELAY_MS: u64 = 500;

/// Tauri 애플리케이션을 초기화하고 실행한다.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::project::get_projects,
            commands::project::add_project,
            commands::project::update_project,
            commands::project::delete_project,
            commands::project::reorder_projects,
            commands::terminal::launch_terminal,
            commands::terminal::get_terminals,
            commands::terminal::focus_terminal,
            commands::terminal::close_terminal,
            commands::terminal::rename_terminal,
            commands::terminal::acknowledge_notification,
            commands::terminal::toggle_terminal_notification,
            commands::terminal::discover_external_terminals,
            commands::terminal::attach_terminal,
            commands::terminal::import_external_tab,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::debug_log::get_debug_logs,
            commands::debug_log::clear_debug_logs,
            commands::theme::get_available_themes,
            commands::theme::get_theme,
            commands::theme::save_theme,
            commands::theme::delete_theme,
            commands::hooks::get_hook_status,
            commands::hooks::register_hooks,
            commands::hooks::unregister_hooks,
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            store::init_data_dir(&data_dir)?;

            // 0. DebugLog 생성
            let debug_log = Arc::new(DebugLog::new(app.handle().clone()));

            // 1. ProcessManager 생성
            let mut pm = ProcessManager::new(Arc::clone(&debug_log));

            // 2. 설정 로드
            let settings = store::load_settings(&data_dir);

            // 3. Notifier 생성
            let notifier = Arc::new(Mutex::new(notifier::Notifier::new(&settings.language)));

            // 4. Reminder 생성
            let reminder = Arc::new(Mutex::new(Reminder::new(
                app.handle().clone(),
                settings.reminder.enabled,
                settings.reminder.interval_minutes,
                settings.reminder.max_repeat,
                &settings.language,
            )));

            // Always on Top 설정 적용
            if settings.always_on_top {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.set_always_on_top(true) {
                        eprintln!("Failed to set always-on-top: {e}");
                    }
                }
            }

            // 3.5. ProcessManager에 Reminder 연결 (close/모니터링에서 사용)
            pm.set_reminder(Arc::clone(&reminder));

            // 4. 모니터링 시작 (✳ 탭 제목 기반 감지)
            pm.start_monitoring(
                app.handle().clone(),
                Arc::clone(&notifier),
                Arc::clone(&reminder),
            );

            // 5. 외부 터미널 자동 어태치 (백그라운드)
            if settings.auto_attach_enabled {
                let projects_data = store::load_projects_from_path(&data_dir).unwrap_or_default();
                let language = settings.language.clone();
                let app_handle_for_attach = app.handle().clone();
                let debug_log_for_attach = Arc::clone(&debug_log);
                // auto_attach는 ProcessManager에 접근해야 하므로 state 등록 후 실행
                let auto_attach_projects = projects_data.projects;
                let auto_attach_language = language;
                app.manage(pm);

                let app_handle_clone = app_handle_for_attach.clone();
                tauri::async_runtime::spawn(async move {
                    // 약간의 지연 후 실행 (모니터링 안정화)
                    tokio::time::sleep(std::time::Duration::from_millis(AUTO_ATTACH_DELAY_MS)).await;
                    let pm = app_handle_clone.state::<ProcessManager>();
                    match pm.auto_attach_on_startup(true, &auto_attach_projects, &auto_attach_language) {
                        Ok(instances) if !instances.is_empty() => {
                            debug_log_for_attach.push("AutoAttach", format!(
                                "Auto-attached {} terminal(s) on startup", instances.len()
                            ));
                            if let Err(e) = app_handle_clone.emit(events::TERMINAL_AUTO_ATTACHED, &instances) {
                                debug_log_for_attach.push("AutoAttach", format!("Failed to emit terminal-auto-attached: {e}"));
                            }
                        }
                        Ok(_) => {
                            debug_log_for_attach.push("AutoAttach", "No matching terminals found on startup".to_string());
                        }
                        Err(e) => {
                            debug_log_for_attach.push("AutoAttach", format!("Auto-attach failed: {e}"));
                        }
                    }
                });
            } else {
                // Tauri state에 등록
                app.manage(pm);
            }
            app.manage(Arc::clone(&notifier));
            app.manage(Arc::clone(&reminder));
            app.manage(Arc::clone(&debug_log));

            // 5. EventWatcher 시작 (Hook 기반 감지)
            let events_dir = services::event_watcher::default_events_dir();
            let app_handle = app.handle().clone();
            let debug_log_for_watcher = Arc::clone(&debug_log);

            match EventWatcher::start(&events_dir, debug_log_for_watcher, move |event| {
                let pm = app_handle.state::<ProcessManager>();
                let transitions =
                    pm.handle_hook_event(&event.session_id, &event.cwd, &event.hook_event_name);

                let notifier_state = app_handle.state::<Arc<Mutex<notifier::Notifier>>>();
                let reminder_state = app_handle.state::<Arc<Mutex<Reminder>>>();

                services::dispatch_transitions(&app_handle, &notifier_state, &reminder_state, &transitions);
            }) {
                Ok(watcher) => {
                    // EventWatcher를 state에 저장하여 Drop 방지
                    app.manage(watcher);
                }
                Err(e) => {
                    debug_log.push("EventWatcher", format!("Failed to start: {e} (hook detection disabled)"));
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("error while running tauri application: {e}");
        });
}
