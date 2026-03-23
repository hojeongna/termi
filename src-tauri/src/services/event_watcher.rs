use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

use crate::services::debug_log::DebugLog;

/// 파일이 완전히 쓰여질 때까지 대기하는 시간 (밀리초)
const FILE_WRITE_SETTLE_MS: u64 = 50;

/// 디바운스 윈도우 (밀리초) — 이 시간 내 중복 이벤트를 하나로 합침
const DEBOUNCE_MS: u64 = 300;

/// Hook 이벤트 파일에서 파싱된 데이터
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HookEvent {
    /// Claude Code 세션 고유 ID — 터미널별 1:1 매핑에 사용
    #[serde(alias = "session_id")]
    pub(crate) session_id: String,
    /// Claude Code가 실행 중인 작업 디렉토리
    pub(crate) cwd: String,
    /// Hook 이벤트 타입: "Stop" 또는 "UserPromptSubmit"
    #[serde(alias = "hook_event_name")]
    pub(crate) hook_event_name: String,
}

/// events 디렉토리를 감시하여 hook 이벤트를 수신하는 서비스.
/// Drop 시 감시 자동 중단.
pub(crate) struct EventWatcher {
    _watcher: RecommendedWatcher,
}

/// JSON 문자열에서 HookEvent를 파싱한다.
pub(crate) fn parse_hook_event(content: &str) -> Result<HookEvent, String> {
    serde_json::from_str::<HookEvent>(content).map_err(|e| format!("JSON parse error: {e}"))
}

/// events 디렉토리에서 이벤트 파일을 읽고 파싱한 후 삭제한다.
pub(crate) fn process_event_file(
    path: &Path,
    debug_log: &Arc<DebugLog>,
) -> Option<HookEvent> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            debug_log.push(
                "EventWatcher",
                format!("Failed to read event file {:?}: {e}", path),
            );
            return None;
        }
    };

    // 읽은 후 파일 삭제
    if let Err(e) = std::fs::remove_file(path) {
        debug_log.push(
            "EventWatcher",
            format!("Failed to delete event file {:?}: {e}", path),
        );
    }

    match parse_hook_event(&content) {
        Ok(event) => {
            debug_log.push(
                "EventWatcher",
                format!(
                    "Parsed event: hook_event_name={}, cwd={}",
                    event.hook_event_name, event.cwd
                ),
            );
            Some(event)
        }
        Err(e) => {
            debug_log.push(
                "EventWatcher",
                format!("Failed to parse event file {:?}: {e}", path),
            );
            None
        }
    }
}

/// events 디렉토리의 기본 경로를 반환한다: `%APPDATA%/termi/events/`
pub(crate) fn default_events_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("termi").join("events")
}

impl EventWatcher {
    /// events 디렉토리 감시를 시작한다.
    /// 디렉토리가 없으면 자동 생성한다.
    /// 파일 생성 감지 시 JSON을 파싱하여 콜백으로 전달한다.
    pub(crate) fn start<F>(
        events_dir: &Path,
        debug_log: Arc<DebugLog>,
        on_event: F,
    ) -> Result<Self, String>
    where
        F: Fn(HookEvent) + Send + Sync + 'static,
    {
        // 디렉토리 생성
        if let Err(e) = std::fs::create_dir_all(events_dir) {
            return Err(format!("Failed to create events directory: {e}"));
        }

        debug_log.push(
            "EventWatcher",
            format!("Watching events directory: {:?}", events_dir),
        );

        let debug_log_clone = Arc::clone(&debug_log);

        let on_event = Arc::new(on_event);

        // 디바운스 상태: 파일별 마지막 이벤트 수신 시각을 추적
        let pending: Arc<Mutex<HashMap<PathBuf, Instant>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Create(_)) {
                        for path in &event.paths {
                            if path.extension().is_some_and(|ext| ext == "json") {
                                let now = Instant::now();
                                let already_scheduled = {
                                    let mut map = pending.lock().unwrap_or_else(|e| e.into_inner());
                                    let is_new = !map.contains_key(path);
                                    map.insert(path.clone(), now);
                                    !is_new
                                };

                                // 이미 이 파일에 대한 디바운스 스레드가 동작 중이면
                                // 타임스탬프만 갱신하고 새 스레드를 생성하지 않는다.
                                if already_scheduled {
                                    continue;
                                }

                                let path = path.clone();
                                let debug_log = Arc::clone(&debug_log_clone);
                                let on_event = Arc::clone(&on_event);
                                let pending = Arc::clone(&pending);
                                std::thread::spawn(move || {
                                    // 디바운스 루프: DEBOUNCE_MS 동안 새 이벤트가 없을 때까지 대기
                                    loop {
                                        std::thread::sleep(std::time::Duration::from_millis(DEBOUNCE_MS));
                                        let should_fire = {
                                            let map = pending.lock().unwrap_or_else(|e| e.into_inner());
                                            match map.get(&path) {
                                                Some(&last) => last.elapsed().as_millis() >= u128::from(DEBOUNCE_MS),
                                                None => true, // 이미 제거됨 → 처리하지 않음
                                            }
                                        };
                                        if should_fire {
                                            break;
                                        }
                                        // 아직 디바운스 윈도우 내에 새 이벤트가 도착함 → 재대기
                                    }

                                    // pending 맵에서 제거
                                    {
                                        let mut map = pending.lock().unwrap_or_else(|e| e.into_inner());
                                        map.remove(&path);
                                    }

                                    // 파일이 완전히 쓰여질 때까지 잠시 대기
                                    std::thread::sleep(std::time::Duration::from_millis(FILE_WRITE_SETTLE_MS));
                                    if let Some(hook_event) =
                                        process_event_file(&path, &debug_log)
                                    {
                                        on_event(hook_event);
                                    }
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    debug_log_clone.push(
                        "EventWatcher",
                        format!("Watch error: {e}"),
                    );
                }
            }
        })
        .map_err(|e| format!("Failed to create watcher: {e}"))?;

        watcher
            .watch(events_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to start watching: {e}"))?;

        Ok(EventWatcher { _watcher: watcher })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn parse_stop_hook_event_from_valid_json() {
        let json = r#"{
            "session_id": "abc-123",
            "transcript_path": "/tmp/transcript",
            "cwd": "C:\\projects\\my-project",
            "hook_event_name": "Stop",
            "stop_hook_active": false,
            "last_assistant_message": "Done!"
        }"#;

        let event = parse_hook_event(json).unwrap();
        assert_eq!(event.session_id, "abc-123");
        assert_eq!(event.cwd, "C:\\projects\\my-project");
        assert_eq!(event.hook_event_name, "Stop");
    }

    #[test]
    fn parse_user_prompt_submit_event_from_valid_json() {
        let json = r#"{
            "session_id": "abc-456",
            "transcript_path": "/tmp/transcript",
            "cwd": "C:\\projects\\another-project",
            "hook_event_name": "UserPromptSubmit"
        }"#;

        let event = parse_hook_event(json).unwrap();
        assert_eq!(event.cwd, "C:\\projects\\another-project");
        assert_eq!(event.hook_event_name, "UserPromptSubmit");
    }

    #[test]
    fn parse_hook_event_fails_on_invalid_json() {
        let result = parse_hook_event("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_hook_event_fails_when_cwd_missing() {
        let json = r#"{"session_id":"s1","hook_event_name": "Stop"}"#;
        let result = parse_hook_event(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_hook_event_fails_when_hook_event_name_missing() {
        let json = r#"{"session_id":"s1","cwd": "C:\\projects\\test"}"#;
        let result = parse_hook_event(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_hook_event_fails_when_session_id_missing() {
        let json = r#"{"cwd":"C:\\test","hook_event_name":"Stop"}"#;
        let result = parse_hook_event(json);
        assert!(result.is_err());
    }

    #[test]
    fn process_event_file_reads_parses_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("stop-20260314-12345.json");
        std::fs::write(
            &file_path,
            r#"{"session_id":"s1","cwd":"C:\\projects\\test","hook_event_name":"Stop"}"#,
        )
        .unwrap();

        let debug_log = Arc::new(DebugLog::new_for_test());
        let event = process_event_file(&file_path, &debug_log).unwrap();

        assert_eq!(event.cwd, "C:\\projects\\test");
        assert_eq!(event.hook_event_name, "Stop");
        assert!(!file_path.exists(), "event file should be deleted after processing");
    }

    #[test]
    fn process_event_file_returns_none_for_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("bad.json");
        std::fs::write(&file_path, "not json").unwrap();

        let debug_log = Arc::new(DebugLog::new_for_test());
        let result = process_event_file(&file_path, &debug_log);

        assert!(result.is_none());
    }

    #[test]
    fn process_event_file_returns_none_for_missing_file() {
        let debug_log = Arc::new(DebugLog::new_for_test());
        let result =
            process_event_file(Path::new("nonexistent-file.json"), &debug_log);

        assert!(result.is_none());
    }

    #[test]
    fn event_watcher_creates_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let events_dir = dir.path().join("subdir").join("events");
        assert!(!events_dir.exists());

        let debug_log = Arc::new(DebugLog::new_for_test());
        let _watcher = EventWatcher::start(&events_dir, debug_log, |_| {}).unwrap();

        assert!(events_dir.exists());
    }

    #[test]
    fn event_watcher_detects_new_json_file_and_calls_callback() {
        let dir = tempfile::tempdir().unwrap();
        let events_dir = dir.path().join("events");

        let (tx, rx) = mpsc::channel();
        let debug_log = Arc::new(DebugLog::new_for_test());

        let _watcher = EventWatcher::start(&events_dir, debug_log, move |event| {
            tx.send(event).unwrap();
        })
        .unwrap();

        // 파일 생성 → watcher가 감지해야 함
        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(
            events_dir.join("stop-test.json"),
            r#"{"session_id":"s1","cwd":"C:\\projects\\hello","hook_event_name":"Stop"}"#,
        )
        .unwrap();

        let event = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(event.cwd, "C:\\projects\\hello");
        assert_eq!(event.hook_event_name, "Stop");
    }

    #[test]
    fn event_watcher_ignores_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        let events_dir = dir.path().join("events");

        let (tx, rx) = mpsc::channel();
        let debug_log = Arc::new(DebugLog::new_for_test());

        let _watcher = EventWatcher::start(&events_dir, debug_log, move |event| {
            tx.send(event).unwrap();
        })
        .unwrap();

        std::thread::sleep(Duration::from_millis(100));
        // .txt 파일은 무시해야 함
        std::fs::write(
            events_dir.join("not-json.txt"),
            r#"{"session_id":"s1","cwd":"C:\\test","hook_event_name":"Stop"}"#,
        )
        .unwrap();

        let result = rx.recv_timeout(Duration::from_secs(1));
        assert!(result.is_err(), "non-json files should be ignored");
    }
}
