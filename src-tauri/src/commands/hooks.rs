use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

const HOOK_SCRIPT_NAME: &str = "termi-hook.ps1";
const HOOK_EVENT_STOP: &str = "Stop";
const HOOK_EVENT_USER_PROMPT_SUBMIT: &str = "UserPromptSubmit";
const CLAUDE_SETTINGS_DIR: &str = ".claude";
const CLAUDE_SETTINGS_FILE: &str = "settings.json";
const HOOK_COMMAND_KEY: &str = "command";
const HOOK_MARKER: &str = "termi-hook";
const USERPROFILE_ENV: &str = "USERPROFILE";

/// Hook 등록 상태 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HookStatus {
    pub(crate) registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hook_path: Option<String>,
}

/// Claude Code settings.json의 최소 타입 표현.
/// `hooks` 이외의 최상위 키는 `rest`로 보존한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hooks: Option<HooksMap>,
    #[serde(flatten)]
    rest: serde_json::Map<String, serde_json::Value>,
}

/// 이벤트 이름 → 이벤트 항목 배열 매핑 (예: `"Stop" → [...]`).
type HooksMap = std::collections::HashMap<String, Vec<HookEventEntry>>;

/// 하나의 hook 이벤트 항목 (`matcher` + 내부 `hooks` 배열).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HookEventEntry {
    matcher: String,
    hooks: Vec<HookItem>,
}

/// 개별 hook 항목 (`type` + `command`).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HookItem {
    #[serde(rename = "type")]
    hook_type: String,
    command: String,
}

/// Claude Code settings.json에서 termi-hook.cmd가 등록되어 있는지 확인한다.
///
/// # Errors
/// * [`AppError::Validation`] - USERPROFILE 환경변수 미설정
/// * [`AppError::Io`] - settings.json 읽기 실패
/// * [`AppError::Json`] - settings.json 파싱 실패
#[tauri::command]
pub(crate) async fn get_hook_status() -> Result<HookStatus, AppError> {
    get_hook_status_impl()
}

/// Claude Code settings.json에 Stop + UserPromptSubmit hook을 등록한다.
///
/// # Errors
/// * [`AppError::Validation`] - hook 스크립트를 찾을 수 없거나 USERPROFILE 미설정
/// * [`AppError::Io`] - settings.json 읽기/쓰기 실패
/// * [`AppError::Json`] - settings.json 파싱/직렬화 실패
#[tauri::command]
pub(crate) async fn register_hooks() -> Result<HookStatus, AppError> {
    register_hooks_impl()
}

/// Claude Code settings.json에서 termi-hook.cmd를 포함하는 hook 항목을 제거한다.
///
/// # Errors
/// * [`AppError::Validation`] - USERPROFILE 환경변수 미설정
/// * [`AppError::Io`] - settings.json 읽기/쓰기 또는 백업 생성 실패
/// * [`AppError::Json`] - settings.json 파싱/직렬화 실패
#[tauri::command]
pub(crate) async fn unregister_hooks() -> Result<HookStatus, AppError> {
    unregister_hooks_impl()
}

/// termi-hook.ps1의 실행 커맨드를 생성한다.
/// Claude Code는 bash로 hook을 실행하므로 forward slash 경로 + powershell 직접 호출.
fn resolve_hook_command() -> Result<String, AppError> {
    let exe_path = env::current_exe().map_err(AppError::Io)?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| AppError::Validation("Cannot determine exe directory".to_string()))?;

    // 개발 환경: src-tauri/target/debug/ → 프로젝트 루트/scripts/
    // 프로덕션: 앱 설치 디렉토리/scripts/
    let candidates = [
        exe_dir.join("scripts").join(HOOK_SCRIPT_NAME),
        exe_dir
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|root| root.join("scripts").join(HOOK_SCRIPT_NAME))
            .unwrap_or_default(),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            // Validate that the resolved script is within the application directory tree
            let canonical = candidate.canonicalize().map_err(AppError::Io)?;
            let exe_dir_canonical = exe_dir.canonicalize().map_err(AppError::Io)?;
            // Also check against project root (for dev environment)
            let allowed_roots = [
                exe_dir_canonical.clone(),
                exe_dir_canonical
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| exe_dir_canonical.clone()),
            ];
            let is_allowed = allowed_roots
                .iter()
                .any(|root| canonical.starts_with(root));
            if !is_allowed {
                return Err(AppError::Validation(format!(
                    "Hook script outside allowed directory: {:?}",
                    canonical
                )));
            }

            // forward slash 사용 — Claude Code가 bash로 실행하므로 backslash 불가
            let ps1_path = candidate.to_string_lossy().replace('\\', "/");
            return Ok(format!(
                "powershell -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
                ps1_path
            ));
        }
    }

    Err(AppError::Validation(
        format!("{} not found", HOOK_SCRIPT_NAME),
    ))
}

/// Claude Code settings.json 경로를 반환한다.
fn claude_settings_path() -> Result<PathBuf, AppError> {
    // Claude Code settings are in ~/.claude/, not in Termi's app_data_dir
    let userprofile = env::var(USERPROFILE_ENV)
        .map_err(|_| AppError::Validation(format!("{} not set", USERPROFILE_ENV)))?;
    Ok(PathBuf::from(userprofile)
        .join(CLAUDE_SETTINGS_DIR)
        .join(CLAUDE_SETTINGS_FILE))
}

/// settings.json을 읽어 `ClaudeSettings`로 반환한다. 없으면 기본값.
fn read_claude_settings() -> Result<ClaudeSettings, AppError> {
    let path = claude_settings_path()?;
    if !path.exists() {
        return Ok(ClaudeSettings {
            hooks: None,
            rest: serde_json::Map::new(),
        });
    }
    let content = fs::read_to_string(&path).map_err(AppError::Io)?;
    serde_json::from_str(&content).map_err(AppError::Json)
}

/// settings.json을 원자적으로 기록한다 (tmp → rename).
fn write_claude_settings(settings: &ClaudeSettings) -> Result<(), AppError> {
    let path = claude_settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(AppError::Io)?;
    }
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(settings).map_err(AppError::Json)?;
    fs::write(&tmp, &content).map_err(AppError::Io)?;
    fs::rename(&tmp, &path).map_err(AppError::Io)?;
    Ok(())
}

/// settings.json에서 termi-hook이 포함된 hook command를 찾아 반환한다.
fn find_termi_hook_in_settings(settings: &ClaudeSettings) -> Option<String> {
    let hooks_map = settings.hooks.as_ref()?;
    for event_type in &[HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT] {
        if let Some(entries) = hooks_map.get(*event_type) {
            for entry in entries {
                for hook in &entry.hooks {
                    if hook.command.to_lowercase().contains(HOOK_MARKER) {
                        return Some(hook.command.clone());
                    }
                }
            }
        }
    }
    None
}

/// Checks hook registration status by examining Claude Code settings
fn get_hook_status_impl() -> Result<HookStatus, AppError> {
    let settings = read_claude_settings()?;
    let hook_path = find_termi_hook_in_settings(&settings);
    Ok(HookStatus {
        registered: hook_path.is_some(),
        hook_path,
    })
}

/// Registers termi-hook in Claude Code's hook settings
fn register_hooks_impl() -> Result<HookStatus, AppError> {
    let hook_command = resolve_hook_command()?;
    let mut settings = read_claude_settings()?;

    // 이미 등록되어 있으면 중복 추가 안 함
    if find_termi_hook_in_settings(&settings).is_some() {
        return Ok(HookStatus {
            registered: true,
            hook_path: Some(hook_command),
        });
    }

    let hooks_map = settings.hooks.get_or_insert_with(std::collections::HashMap::new);

    let termi_entry = HookEventEntry {
        matcher: String::new(),
        hooks: vec![HookItem {
            hook_type: HOOK_COMMAND_KEY.to_string(),
            command: hook_command.clone(),
        }],
    };

    for event_type in &[HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT] {
        hooks_map
            .entry((*event_type).to_string())
            .or_default()
            .push(termi_entry.clone());
    }

    write_claude_settings(&settings)?;

    Ok(HookStatus {
        registered: true,
        hook_path: Some(hook_command),
    })
}

/// Removes termi-hook from Claude Code's hook settings
fn unregister_hooks_impl() -> Result<HookStatus, AppError> {
    let settings_path = claude_settings_path()?;

    if !settings_path.exists() {
        return Ok(HookStatus {
            registered: false,
            hook_path: None,
        });
    }

    // 백업 생성
    let backup_path = settings_path.with_extension("json.bak");
    fs::copy(&settings_path, &backup_path).map_err(AppError::Io)?;

    let mut settings = read_claude_settings()?;

    if let Some(hooks_map) = settings.hooks.as_mut() {
        for event_type in &[HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT] {
            if let Some(entries) = hooks_map.get_mut(*event_type) {
                // termi-hook을 포함하는 항목은 제거
                entries.retain(|entry| {
                    !entry.hooks.iter().any(|hook| {
                        hook.command.to_lowercase().contains(HOOK_MARKER)
                    })
                });

                // 배열이 비면 키 제거
                if entries.is_empty() {
                    hooks_map.remove(*event_type);
                }
            }
        }

        // hooks 객체가 비면 키 제거
        if hooks_map.is_empty() {
            settings.hooks = None;
        }
    }

    write_claude_settings(&settings)?;

    Ok(HookStatus {
        registered: false,
        hook_path: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hook_entry(hook_path: &str) -> HookEventEntry {
        HookEventEntry {
            matcher: String::new(),
            hooks: vec![HookItem {
                hook_type: HOOK_COMMAND_KEY.to_string(),
                command: hook_path.to_string(),
            }],
        }
    }

    fn make_settings_with_hooks(hook_path: &str) -> ClaudeSettings {
        let entry = make_hook_entry(hook_path);
        let mut hooks_map = std::collections::HashMap::new();
        hooks_map.insert(HOOK_EVENT_STOP.to_string(), vec![entry.clone()]);
        hooks_map.insert(HOOK_EVENT_USER_PROMPT_SUBMIT.to_string(), vec![entry]);
        ClaudeSettings {
            hooks: Some(hooks_map),
            rest: serde_json::Map::new(),
        }
    }

    #[test]
    fn find_termi_hook_detects_registered_hook() {
        let settings = make_settings_with_hooks("C:\\path\\to\\termi-hook.cmd");
        let result = find_termi_hook_in_settings(&settings);
        assert!(result.is_some());
        assert!(result.unwrap().contains(HOOK_MARKER));
    }

    #[test]
    fn find_termi_hook_returns_none_when_not_registered() {
        let entry = make_hook_entry("other-hook.cmd");
        let mut hooks_map = std::collections::HashMap::new();
        hooks_map.insert(HOOK_EVENT_STOP.to_string(), vec![entry]);
        let settings = ClaudeSettings {
            hooks: Some(hooks_map),
            rest: serde_json::Map::new(),
        };
        assert!(find_termi_hook_in_settings(&settings).is_none());
    }

    #[test]
    fn find_termi_hook_returns_none_for_empty_settings() {
        let settings = ClaudeSettings {
            hooks: None,
            rest: serde_json::Map::new(),
        };
        assert!(find_termi_hook_in_settings(&settings).is_none());
    }

    #[test]
    fn find_termi_hook_case_insensitive() {
        let settings = make_settings_with_hooks("C:\\Path\\To\\TERMI-HOOK.CMD");
        assert!(find_termi_hook_in_settings(&settings).is_some());
    }

    #[test]
    fn hook_status_serializes_to_camel_case() {
        let status = HookStatus {
            registered: true,
            hook_path: Some("C:\\path\\to\\hook.cmd".to_string()),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"registered\":true"));
        assert!(json.contains("\"hookPath\""));
    }

    #[test]
    fn unregister_removes_only_termi_hooks_preserving_others() {
        let other_entry = make_hook_entry("C:\\other\\hook.cmd");
        let termi_entry = make_hook_entry("C:\\path\\termi-hook.cmd");

        let mut hooks_map = std::collections::HashMap::new();
        hooks_map.insert(
            HOOK_EVENT_STOP.to_string(),
            vec![other_entry, termi_entry.clone()],
        );
        hooks_map.insert(
            HOOK_EVENT_USER_PROMPT_SUBMIT.to_string(),
            vec![termi_entry],
        );
        let mut rest = serde_json::Map::new();
        rest.insert("other_key".to_string(), serde_json::Value::String("preserved".to_string()));
        let mut settings = ClaudeSettings {
            hooks: Some(hooks_map),
            rest,
        };

        // Simulate unregister logic (same as unregister_hooks_impl but on in-memory value)
        if let Some(hooks_map) = settings.hooks.as_mut() {
            for event_type in &[HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT] {
                if let Some(entries) = hooks_map.get_mut(*event_type) {
                    entries.retain(|entry| {
                        !entry.hooks.iter().any(|hook| {
                            hook.command.to_lowercase().contains(HOOK_MARKER)
                        })
                    });
                    if entries.is_empty() {
                        hooks_map.remove(*event_type);
                    }
                }
            }
            if hooks_map.is_empty() {
                settings.hooks = None;
            }
        }

        // Stop 배열에 other hook만 남아야 함
        let hooks_map = settings.hooks.as_ref().unwrap();
        let stop_hooks = hooks_map.get(HOOK_EVENT_STOP).unwrap();
        assert_eq!(stop_hooks.len(), 1);
        assert!(stop_hooks[0].hooks[0].command.contains("other"));

        // UserPromptSubmit은 비어서 제거됨
        assert!(hooks_map.get(HOOK_EVENT_USER_PROMPT_SUBMIT).is_none());

        // other_key는 보존됨
        assert_eq!(settings.rest.get("other_key").unwrap(), "preserved");
    }

    #[test]
    fn register_creates_correct_hook_structure() {
        let mut settings = ClaudeSettings {
            hooks: None,
            rest: serde_json::Map::new(),
        };
        let hook_path = "C:\\test\\termi-hook.cmd";

        // Simulate register logic
        let hooks_map = settings.hooks.get_or_insert_with(std::collections::HashMap::new);
        let termi_entry = make_hook_entry(hook_path);

        for event_type in &[HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT] {
            hooks_map
                .entry(event_type.to_string())
                .or_default()
                .push(termi_entry.clone());
        }

        // 검증
        let hooks_map = settings.hooks.as_ref().unwrap();
        let stop = hooks_map.get(HOOK_EVENT_STOP).unwrap();
        assert_eq!(stop.len(), 1);
        assert_eq!(stop[0].matcher, "");
        assert_eq!(stop[0].hooks[0].hook_type, HOOK_COMMAND_KEY);
        assert_eq!(stop[0].hooks[0].command, hook_path);

        let prompt = hooks_map.get(HOOK_EVENT_USER_PROMPT_SUBMIT).unwrap();
        assert_eq!(prompt.len(), 1);
        assert_eq!(prompt[0].hooks[0].command, hook_path);
    }

    #[test]
    fn register_preserves_existing_hooks() {
        let existing_entry = make_hook_entry("C:\\existing\\hook.cmd");
        let mut hooks_map_init = std::collections::HashMap::new();
        hooks_map_init.insert(HOOK_EVENT_STOP.to_string(), vec![existing_entry]);
        let mut settings = ClaudeSettings {
            hooks: Some(hooks_map_init),
            rest: serde_json::Map::new(),
        };

        let hook_path = "C:\\test\\termi-hook.cmd";
        let hooks_map = settings.hooks.as_mut().unwrap();
        let termi_entry = make_hook_entry(hook_path);

        for event_type in &[HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT] {
            hooks_map
                .entry(event_type.to_string())
                .or_default()
                .push(termi_entry.clone());
        }

        // Stop에 기존 hook + termi hook = 2개
        let hooks_map = settings.hooks.as_ref().unwrap();
        let stop = hooks_map.get(HOOK_EVENT_STOP).unwrap();
        assert_eq!(stop.len(), 2);
        assert!(stop[0].hooks[0].command.contains("existing"));
        assert!(stop[1].hooks[0].command.contains(HOOK_MARKER));
    }
}
