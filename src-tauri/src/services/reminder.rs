use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::AppHandle;

use crate::models::terminal::Activity;
use crate::services::notifier;

const SECONDS_PER_MINUTE: u64 = 60;

struct ActiveReminder {
    cancel: Arc<AtomicBool>,
}

/// 미확인 알림에 대해 주기적으로 리마인더를 발생시키는 서비스.
pub(crate) struct Reminder {
    app_handle: Option<AppHandle>,
    enabled: bool,
    interval_minutes: u32,
    max_repeat: u32,
    language: String,
    active: HashMap<String, ActiveReminder>,
}

impl Reminder {
    /// 새 Reminder 인스턴스를 생성한다.
    pub(crate) fn new(app_handle: AppHandle, enabled: bool, interval_minutes: u32, max_repeat: u32, language: &str) -> Self {
        Self {
            app_handle: Some(app_handle),
            enabled,
            interval_minutes,
            max_repeat,
            language: language.to_string(),
            active: HashMap::new(),
        }
    }

    /// 테스트용 Reminder를 생성한다. AppHandle 없이 stop/has_active 등만 사용 가능.
    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        Self {
            app_handle: None,
            enabled: true,
            interval_minutes: 1,
            max_repeat: 3,
            language: "en".to_string(),
            active: HashMap::new(),
        }
    }

    /// 특정 터미널의 리마인더를 시작한다.
    pub(crate) fn start_reminder(
        &mut self,
        terminal_id: &str,
        project_name: &str,
        terminal_name: &str,
        notifier: &Arc<Mutex<notifier::Notifier>>,
    ) {
        if !self.enabled {
            return;
        }

        // 기존 리마인더 중지 후 재시작
        self.stop_reminder(terminal_id);

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancel);
        let interval = Duration::from_secs(self.interval_minutes as u64 * SECONDS_PER_MINUTE);
        let max_repeat = self.max_repeat;
        let app = match self.app_handle.clone() {
            Some(handle) => handle,
            None => return, // 테스트 환경에서는 스레드 생성 불필요
        };
        let tid = terminal_id.to_string();
        let pname = project_name.to_string();
        let tname = terminal_name.to_string();
        let lang = self.language.clone();
        let notifier_clone = Arc::clone(notifier);

        tauri::async_runtime::spawn(async move {
            let mut count = 0u32;
            let reminder_label = if lang == "ko" { "리마인더" } else { "Reminder" };
            loop {
                tokio::time::sleep(interval).await;
                if cancel_clone.load(Ordering::Relaxed) {
                    return;
                }

                count += 1;
                if let Ok(mut n) = notifier_clone.lock() {
                    let reminder_name = format!("{} ({} {}/{})", pname, reminder_label, count, max_repeat);
                    let reminder_tname = format!("{} ({})", tname, reminder_label);
                    let _ = n.send_status_notification(&app, &tid, &reminder_name, &reminder_tname, &Activity::Idle);
                }

                if max_repeat > 0 && count >= max_repeat {
                    return;
                }
            }
        });

        self.active.insert(terminal_id.to_string(), ActiveReminder { cancel });
    }

    /// 특정 터미널의 리마인더를 중지한다.
    pub(crate) fn stop_reminder(&mut self, terminal_id: &str) {
        if let Some(reminder) = self.active.remove(terminal_id) {
            reminder.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 모든 리마인더를 중지한다.
    pub(crate) fn stop_all(&mut self) {
        for (_, reminder) in self.active.drain() {
            reminder.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 설정 변경을 반영한다.
    pub(crate) fn update_settings(&mut self, enabled: bool, interval_minutes: u32, max_repeat: u32, language: &str) {
        self.enabled = enabled;
        self.interval_minutes = interval_minutes;
        self.max_repeat = max_repeat;
        self.language = language.to_string();

        if !enabled {
            self.stop_all();
        }
    }

    /// 특정 터미널에 대한 활성 리마인더가 있는지 확인한다.
    #[cfg(test)]
    pub(crate) fn has_active(&self, terminal_id: &str) -> bool {
        self.active.contains_key(terminal_id)
    }

    /// 활성 리마인더의 수를 반환한다.
    #[cfg(test)]
    pub(crate) fn active_count(&self) -> usize {
        self.active.len()
    }

    /// 테스트용: 스레드를 생성하지 않고 리마인더 엔트리만 등록한다.
    #[cfg(test)]
    pub(crate) fn insert_fake_active(&mut self, terminal_id: &str) {
        let cancel = Arc::new(AtomicBool::new(false));
        self.active.insert(terminal_id.to_string(), ActiveReminder { cancel });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_fake_active_and_has_active_and_active_count() {
        let mut reminder = Reminder::new_for_test();
        assert_eq!(reminder.active_count(), 0);
        assert!(!reminder.has_active("t1"));

        reminder.insert_fake_active("t1");
        assert!(reminder.has_active("t1"));
        assert!(!reminder.has_active("t2"));
        assert_eq!(reminder.active_count(), 1);

        reminder.insert_fake_active("t2");
        assert!(reminder.has_active("t2"));
        assert_eq!(reminder.active_count(), 2);
    }

    #[test]
    fn stop_reminder_removes_active_entry() {
        let mut reminder = Reminder::new_for_test();
        reminder.insert_fake_active("t1");
        reminder.insert_fake_active("t2");
        assert_eq!(reminder.active_count(), 2);

        reminder.stop_reminder("t1");
        assert!(!reminder.has_active("t1"));
        assert!(reminder.has_active("t2"));
        assert_eq!(reminder.active_count(), 1);
    }

    #[test]
    fn stop_reminder_on_nonexistent_id_is_noop() {
        let mut reminder = Reminder::new_for_test();
        reminder.insert_fake_active("t1");

        reminder.stop_reminder("nonexistent");
        assert_eq!(reminder.active_count(), 1);
        assert!(reminder.has_active("t1"));
    }

    #[test]
    fn stop_all_clears_all_active_reminders() {
        let mut reminder = Reminder::new_for_test();
        reminder.insert_fake_active("t1");
        reminder.insert_fake_active("t2");
        reminder.insert_fake_active("t3");
        assert_eq!(reminder.active_count(), 3);

        reminder.stop_all();
        assert_eq!(reminder.active_count(), 0);
        assert!(!reminder.has_active("t1"));
        assert!(!reminder.has_active("t2"));
        assert!(!reminder.has_active("t3"));
    }

    #[test]
    fn update_settings_with_enabled_false_stops_all_active() {
        let mut reminder = Reminder::new_for_test();
        reminder.insert_fake_active("t1");
        reminder.insert_fake_active("t2");
        assert_eq!(reminder.active_count(), 2);

        reminder.update_settings(false, 5, 10, "en");
        assert_eq!(reminder.active_count(), 0);
        assert!(!reminder.has_active("t1"));
        assert!(!reminder.has_active("t2"));
    }

    #[test]
    fn update_settings_with_enabled_true_keeps_active() {
        let mut reminder = Reminder::new_for_test();
        reminder.insert_fake_active("t1");
        assert_eq!(reminder.active_count(), 1);

        reminder.update_settings(true, 10, 5, "en");
        assert_eq!(reminder.active_count(), 1);
        assert!(reminder.has_active("t1"));
    }
}
