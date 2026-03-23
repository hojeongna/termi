use std::collections::VecDeque;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::events;

const MAX_LOG_ENTRIES: usize = 1000;

/// 개별 디버그 로그 항목.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogEntry {
    /// 밀리초 단위 Unix 타임스탬프
    pub(crate) timestamp: u64,
    /// 로그 카테고리 (HWND, IO, 상태, 종료 등)
    pub(crate) category: String,
    /// 로그 메시지
    pub(crate) message: String,
}

/// 디버그 로그를 메모리에 보관하는 링 버퍼.
/// 최대 MAX_LOG_ENTRIES 개의 로그를 유지하고, 초과 시 오래된 항목을 제거한다.
pub(crate) struct DebugLog {
    entries: Mutex<VecDeque<LogEntry>>,
    app_handle: Option<tauri::AppHandle>,
}

impl DebugLog {
    /// 빈 디버그 로그 저장소를 생성한다.
    pub(crate) fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)),
            app_handle: Some(app_handle),
        }
    }

    /// 로그 항목을 추가한다. 최대 용량 초과 시 가장 오래된 항목을 제거한다.
    /// 항목 추가 후 `DEBUG_LOG_UPDATED` 이벤트를 emit하여 프론트엔드에 알린다.
    pub(crate) fn push(&self, category: &str, message: String) {
        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() >= MAX_LOG_ENTRIES {
                entries.pop_front();
            }
            entries.push_back(LogEntry {
                timestamp: timestamp_millis(),
                category: category.to_string(),
                message,
            });
        }
        if let Some(handle) = &self.app_handle {
            let _ = handle.emit(events::DEBUG_LOG_UPDATED, ());
        }
    }

    /// 모든 로그 항목을 반환한다.
    pub(crate) fn get_all(&self) -> Vec<LogEntry> {
        self.entries
            .lock()
            .map(|e| e.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 모든 로그 항목을 삭제한다.
    pub(crate) fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

fn timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
impl DebugLog {
    /// 테스트용 생성자 — AppHandle 없이 DebugLog를 생성한다.
    pub(crate) fn new_for_test() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES)),
            app_handle: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_debug_log_is_empty() {
        let log = DebugLog::new_for_test();
        assert!(log.get_all().is_empty());
    }

    #[test]
    fn push_adds_entry() {
        let log = DebugLog::new_for_test();
        log.push("TEST", "hello".to_string());
        let entries = log.get_all();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].category, "TEST");
        assert_eq!(entries[0].message, "hello");
    }

    #[test]
    fn push_respects_max_entries() {
        let log = DebugLog::new_for_test();
        for i in 0..MAX_LOG_ENTRIES + 10 {
            log.push("TEST", format!("msg-{}", i));
        }
        let entries = log.get_all();
        assert_eq!(entries.len(), MAX_LOG_ENTRIES);
        // 가장 오래된 10개가 제거되었으므로 첫 항목은 msg-10
        assert_eq!(entries[0].message, "msg-10");
    }

    #[test]
    fn clear_removes_all_entries() {
        let log = DebugLog::new_for_test();
        log.push("TEST", "a".to_string());
        log.push("TEST", "b".to_string());
        log.clear();
        assert!(log.get_all().is_empty());
    }

    #[test]
    fn timestamp_is_nonzero() {
        let log = DebugLog::new_for_test();
        log.push("TEST", "x".to_string());
        assert!(log.get_all()[0].timestamp > 0);
    }

    #[test]
    fn entries_preserve_order() {
        let log = DebugLog::new_for_test();
        log.push("A", "first".to_string());
        log.push("B", "second".to_string());
        log.push("C", "third".to_string());
        let entries = log.get_all();
        assert_eq!(entries[0].message, "first");
        assert_eq!(entries[1].message, "second");
        assert_eq!(entries[2].message, "third");
    }
}
