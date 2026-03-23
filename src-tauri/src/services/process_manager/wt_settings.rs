use std::path::{Path, PathBuf};

use crate::error::AppError;

/// WT settings.json 경로 탐색 순서
const WT_SETTINGS_CANDIDATES: &[&[&str]] = &[
    // 1. Store 버전
    &[
        "Packages",
        "Microsoft.WindowsTerminal_8wekyb3d8bbwe",
        "LocalState",
        "settings.json",
    ],
    // 2. Preview 버전
    &[
        "Packages",
        "Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe",
        "LocalState",
        "settings.json",
    ],
    // 3. Unpackaged 버전
    &["Microsoft", "Windows Terminal", "settings.json"],
];

/// `%LOCALAPPDATA%`를 기준으로 WT settings.json을 자동 탐지한다.
pub(super) fn find_wt_settings_path() -> Result<PathBuf, AppError> {
    let local_app_data = std::env::var("LOCALAPPDATA").map_err(|_| {
        AppError::Terminal("LOCALAPPDATA 환경변수를 찾을 수 없습니다".into())
    })?;
    find_wt_settings_in(Path::new(&local_app_data))
}

/// 주어진 base 디렉토리에서 WT settings.json을 순차 탐색한다.
fn find_wt_settings_in(base: &Path) -> Result<PathBuf, AppError> {
    for candidate in WT_SETTINGS_CANDIDATES {
        let mut path = base.to_path_buf();
        for segment in *candidate {
            path.push(segment);
        }
        if path.exists() {
            return Ok(path);
        }
    }
    Err(AppError::Terminal(
        "Windows Terminal 설정 파일을 찾을 수 없습니다".into(),
    ))
}

/// WT settings.json을 읽어 JSON Value로 반환한다.
/// WT 기본 settings.json에 포함된 `//` 주석을 제거 후 파싱한다.
pub(super) fn read_wt_settings(path: &Path) -> Result<serde_json::Value, AppError> {
    let content = std::fs::read_to_string(path)?;
    let stripped = strip_json_comments(&content);
    let value: serde_json::Value = serde_json::from_str(&stripped)?;
    Ok(value)
}

/// WT settings.json을 원자적으로 저장한다 (tmp → rename).
pub(super) fn write_wt_settings(
    path: &Path,
    value: &serde_json::Value,
) -> Result<(), AppError> {
    let json_str = serde_json::to_string_pretty(value)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json_str)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// JSON 문자열에서 `//` 라인 주석을 제거한다.
/// 문자열 리터럴 내부의 `//`는 보존한다 (예: URL).
fn strip_json_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for line in input.lines() {
        let trimmed = line.trim();
        // 전체 라인이 주석인 경우 스킵
        if trimmed.starts_with("//") {
            result.push('\n');
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Termi 전용 moveTab 키바인딩 키 조합 (F24는 표준 키보드에 없으므로 충돌 없음)
pub(super) const TERMI_MOVE_TAB_KEYS: &str = "ctrl+shift+alt+f24";

/// WT 핫리로드 대기 시간 (ms)
pub(super) const WT_SETTINGS_HOT_RELOAD_DELAY_MS: u64 = 500;

/// WT settings.json의 `actions` 배열에 Termi 전용 moveTab 키바인딩을 주입/업데이트한다.
pub(super) fn inject_move_tab_keybinding(
    settings_path: &Path,
    target_window: &str,
) -> Result<(), AppError> {
    let mut settings = read_wt_settings(settings_path)?;

    // actions 배열 확보 (없으면 생성)
    if settings.get("actions").is_none() {
        settings["actions"] = serde_json::json!([]);
    }

    let actions = settings["actions"]
        .as_array_mut()
        .ok_or_else(|| AppError::Terminal("actions 필드가 배열이 아닙니다".into()))?;

    // Termi 키바인딩 검색 (keys == TERMI_MOVE_TAB_KEYS)
    let existing = actions.iter_mut().find(|a| {
        a.get("keys")
            .and_then(|k| k.as_str())
            .is_some_and(|k| k == TERMI_MOVE_TAB_KEYS)
    });

    match existing {
        Some(entry) => {
            // window 값만 업데이트
            entry["command"]["window"] = serde_json::Value::String(target_window.into());
        }
        None => {
            // 새 키바인딩 추가
            actions.push(serde_json::json!({
                "command": {
                    "action": "moveTab",
                    "direction": "forward",
                    "window": target_window
                },
                "keys": TERMI_MOVE_TAB_KEYS
            }));
        }
    }

    write_wt_settings(settings_path, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn returns_store_path_when_exists() {
        let tmp = tempdir().unwrap();
        let store_dir = tmp
            .path()
            .join("Packages")
            .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
            .join("LocalState");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::write(store_dir.join("settings.json"), "{}").unwrap();

        let result = find_wt_settings_in(tmp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), store_dir.join("settings.json"));
    }

    #[test]
    fn returns_preview_path_when_store_missing() {
        let tmp = tempdir().unwrap();
        let preview_dir = tmp
            .path()
            .join("Packages")
            .join("Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe")
            .join("LocalState");
        std::fs::create_dir_all(&preview_dir).unwrap();
        std::fs::write(preview_dir.join("settings.json"), "{}").unwrap();

        let result = find_wt_settings_in(tmp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), preview_dir.join("settings.json"));
    }

    #[test]
    fn returns_unpackaged_path_when_others_missing() {
        let tmp = tempdir().unwrap();
        let unpackaged_dir = tmp.path().join("Microsoft").join("Windows Terminal");
        std::fs::create_dir_all(&unpackaged_dir).unwrap();
        std::fs::write(unpackaged_dir.join("settings.json"), "{}").unwrap();

        let result = find_wt_settings_in(tmp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), unpackaged_dir.join("settings.json"));
    }

    #[test]
    fn prefers_store_over_preview() {
        let tmp = tempdir().unwrap();
        // Store
        let store_dir = tmp
            .path()
            .join("Packages")
            .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
            .join("LocalState");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::write(store_dir.join("settings.json"), "{}").unwrap();
        // Preview
        let preview_dir = tmp
            .path()
            .join("Packages")
            .join("Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe")
            .join("LocalState");
        std::fs::create_dir_all(&preview_dir).unwrap();
        std::fs::write(preview_dir.join("settings.json"), "{}").unwrap();

        let result = find_wt_settings_in(tmp.path());
        assert_eq!(result.unwrap(), store_dir.join("settings.json"));
    }

    #[test]
    fn returns_error_when_no_settings_found() {
        let tmp = tempdir().unwrap();
        let result = find_wt_settings_in(tmp.path());
        assert!(result.is_err());
    }

    // --- Task 1.2: read/write/strip_comments ---

    #[test]
    fn strip_json_comments_removes_line_comments() {
        let input = r#"{
    // This is a comment
    "name": "value",
    // Another comment
    "number": 42
}"#;
        let stripped = strip_json_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(parsed["name"], "value");
        assert_eq!(parsed["number"], 42);
    }

    #[test]
    fn strip_json_comments_preserves_urls() {
        let input = r#"{
    "$schema": "https://aka.ms/terminal-profiles-schema",
    "name": "test"
}"#;
        let stripped = strip_json_comments(input);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(
            parsed["$schema"],
            "https://aka.ms/terminal-profiles-schema"
        );
    }

    #[test]
    fn read_wt_settings_parses_json_with_comments() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{
    // Default profile
    "defaultProfile": "{guid}",
    "actions": []
}"#,
        )
        .unwrap();

        let value = read_wt_settings(&path).unwrap();
        assert_eq!(value["defaultProfile"], "{guid}");
        assert!(value["actions"].is_array());
    }

    #[test]
    fn write_wt_settings_creates_valid_json() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        let value = serde_json::json!({
            "defaultProfile": "{guid}",
            "actions": []
        });

        write_wt_settings(&path, &value).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["defaultProfile"], "{guid}");
    }

    // --- Task 2: inject_move_tab_keybinding ---

    #[test]
    fn inject_adds_keybinding_to_empty_actions() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"defaultProfile": "{guid}", "actions": []}"#,
        )
        .unwrap();

        inject_move_tab_keybinding(&path, "termi_abc123").unwrap();

        let value = read_wt_settings(&path).unwrap();
        let actions = value["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["keys"], TERMI_MOVE_TAB_KEYS);
        assert_eq!(actions[0]["command"]["action"], "moveTab");
        assert_eq!(actions[0]["command"]["window"], "termi_abc123");
    }

    #[test]
    fn inject_updates_existing_keybinding_window() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        let initial = serde_json::json!({
            "actions": [{
                "command": { "action": "moveTab", "direction": "forward", "window": "old_window" },
                "keys": TERMI_MOVE_TAB_KEYS
            }]
        });
        std::fs::write(&path, serde_json::to_string(&initial).unwrap()).unwrap();

        inject_move_tab_keybinding(&path, "termi_new").unwrap();

        let value = read_wt_settings(&path).unwrap();
        let actions = value["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["command"]["window"], "termi_new");
    }

    #[test]
    fn inject_preserves_existing_user_keybindings() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        let initial = serde_json::json!({
            "actions": [{
                "command": { "action": "copy" },
                "keys": "ctrl+c"
            }]
        });
        std::fs::write(&path, serde_json::to_string(&initial).unwrap()).unwrap();

        inject_move_tab_keybinding(&path, "termi_abc").unwrap();

        let value = read_wt_settings(&path).unwrap();
        let actions = value["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0]["keys"], "ctrl+c");
        assert_eq!(actions[1]["keys"], TERMI_MOVE_TAB_KEYS);
    }

    #[test]
    fn inject_creates_actions_array_when_missing() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"defaultProfile": "{guid}"}"#).unwrap();

        inject_move_tab_keybinding(&path, "termi_abc").unwrap();

        let value = read_wt_settings(&path).unwrap();
        let actions = value["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["keys"], TERMI_MOVE_TAB_KEYS);
    }

    #[test]
    fn write_wt_settings_overwrites_existing() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        std::fs::write(&path, r#"{"old": true}"#).unwrap();

        let value = serde_json::json!({"new": true});
        write_wt_settings(&path, &value).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.get("old").is_none());
        assert_eq!(parsed["new"], true);
    }
}
