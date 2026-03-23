use std::path::Path;

use serde::{Deserialize, Serialize};

// All fallible functions in this module return `AppError` from `crate::error`.
use crate::error::AppError;
use crate::models::theme::{File, ThemeType, default_dark_theme, default_light_theme};
use crate::store;

const THEMES_DIR_NAME: &str = "themes";
const MAX_THEME_ID_LENGTH: usize = 128;

/// A lightweight theme list entry returned by `get_available_themes`, containing
/// metadata (id, name, type, description) but omitting the full color map.
/// 테마 목록 항목 (색상 제외)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) theme_type: ThemeType,
    pub(crate) description: String,
}

pub(crate) const DEFAULT_DARK_ID: &str = "default-dark";
pub(crate) const DEFAULT_LIGHT_ID: &str = "default-light";

/// theme_id에 경로 구분자나 경로 탈출 문자가 포함되어 있지 않은지 검증한다.
fn validate_theme_id(theme_id: &str) -> Result<(), AppError> {
    if theme_id.is_empty() {
        return Err(AppError::Validation(
            "Theme ID cannot be empty".to_string(),
        ));
    }
    if theme_id.len() > MAX_THEME_ID_LENGTH {
        return Err(AppError::Validation(
            format!("Theme ID must not exceed {} characters", MAX_THEME_ID_LENGTH),
        ));
    }
    if theme_id.contains('/')
        || theme_id.contains('\\')
        || theme_id.contains("..")
    {
        return Err(AppError::Validation(
            "테마 ID에 경로 구분자를 사용할 수 없습니다".to_string(),
        ));
    }
    Ok(())
}

/// theme_id로부터 JSON 파일명을 생성한다.
fn theme_file_name(id: &str) -> String {
    format!("{}.json", id)
}

/// 사용 가능한 모든 테마 목록을 반환한다 (기본 2개 + 커스텀 파일들).
pub(crate) fn list_themes(themes_dir: &Path) -> Vec<ListEntry> {
    let dark = default_dark_theme();
    let light = default_light_theme();

    let mut entries = vec![
        ListEntry {
            id: DEFAULT_DARK_ID.to_string(),
            name: dark.name,
            theme_type: dark.theme_type,
            description: dark.description,
        },
        ListEntry {
            id: DEFAULT_LIGHT_ID.to_string(),
            name: light.name,
            theme_type: light.theme_type,
            description: light.description,
        },
    ];

    if let Ok(dir) = std::fs::read_dir(themes_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(theme) = serde_json::from_str::<File>(&content) {
                        let id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        entries.push(ListEntry {
                            id,
                            name: theme.name,
                            theme_type: theme.theme_type,
                            description: theme.description,
                        });
                    }
                }
            }
        }
    }

    entries
}

/// 특정 테마를 ID로 로드한다.
pub(crate) fn load_theme(themes_dir: &Path, theme_id: &str) -> Result<File, AppError> {
    validate_theme_id(theme_id)?;
    match theme_id {
        DEFAULT_DARK_ID => Ok(default_dark_theme()),
        DEFAULT_LIGHT_ID => Ok(default_light_theme()),
        _ => {
            let path = themes_dir.join(theme_file_name(theme_id));
            let content = std::fs::read_to_string(&path)?;
            let theme: File = serde_json::from_str(&content)?;
            Ok(theme)
        }
    }
}

/// Tauri 커맨드: 사용 가능한 테마 목록 반환
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
///
/// # Errors
/// * [`AppError::Io`] - 데이터 디렉토리 접근 실패
/// * [`AppError::Validation`] - 앱 데이터 경로를 결정할 수 없는 경우
#[tauri::command]
pub(crate) async fn get_available_themes(
    app_handle: tauri::AppHandle,
) -> Result<Vec<ListEntry>, AppError> {
    let themes_dir = get_themes_dir(&app_handle)?;
    let entries = store::spawn_io(move || Ok(list_themes(&themes_dir))).await?;
    Ok(entries)
}

/// Tauri 커맨드: 특정 테마 로드
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
/// * `theme_id` - 로드할 테마의 ID
///
/// # Errors
/// * [`AppError::Validation`] - 테마 ID가 비어 있거나 경로 구분자 포함
/// * [`AppError::Io`] - 커스텀 테마 파일을 읽을 수 없는 경우
/// * [`AppError::Json`] - 테마 JSON 파싱 실패
#[tauri::command]
pub(crate) async fn get_theme(
    app_handle: tauri::AppHandle,
    theme_id: String,
) -> Result<File, AppError> {
    validate_theme_id(&theme_id)?;
    let themes_dir = get_themes_dir(&app_handle)?;
    let theme = store::spawn_io(move || load_theme(&themes_dir, &theme_id)).await?;
    Ok(theme)
}

/// 커스텀 테마를 JSON 파일로 저장한다. theme_id를 파일명으로 사용.
pub(crate) fn save_custom_theme(themes_dir: &Path, theme_id: &str, theme: &File) -> Result<File, AppError> {
    validate_theme_id(theme_id)?;
    if theme_id == DEFAULT_DARK_ID || theme_id == DEFAULT_LIGHT_ID {
        return Err(AppError::Validation("기본 테마는 수정할 수 없습니다".to_string()));
    }
    std::fs::create_dir_all(themes_dir)?;
    let path = themes_dir.join(theme_file_name(theme_id));
    let json = serde_json::to_string_pretty(theme)?;
    crate::store::atomic_write(&path, json.as_bytes())?;
    Ok(theme.clone())
}

/// 커스텀 테마 파일을 삭제한다.
pub(crate) fn delete_custom_theme(themes_dir: &Path, theme_id: &str) -> Result<bool, AppError> {
    validate_theme_id(theme_id)?;
    if theme_id == DEFAULT_DARK_ID || theme_id == DEFAULT_LIGHT_ID {
        return Err(AppError::Validation("기본 테마는 삭제할 수 없습니다".to_string()));
    }
    let path = themes_dir.join(theme_file_name(theme_id));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(true)
}

/// Tauri 커맨드: 커스텀 테마 저장
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
/// * `theme_id` - 저장할 테마의 ID
/// * `theme` - 저장할 테마 데이터
///
/// # Errors
/// * [`AppError::Validation`] - 테마 ID가 유효하지 않거나 기본 테마 ID인 경우
/// * [`AppError::Io`] - 파일 쓰기 실패
/// * [`AppError::Json`] - 테마 직렬화 실패
#[tauri::command]
pub(crate) async fn save_theme(
    app_handle: tauri::AppHandle,
    theme_id: String,
    theme: File,
) -> Result<File, AppError> {
    validate_theme_id(&theme_id)?;
    let themes_dir = get_themes_dir(&app_handle)?;
    let saved = store::spawn_io(move || save_custom_theme(&themes_dir, &theme_id, &theme)).await?;
    Ok(saved)
}

/// Tauri 커맨드: 커스텀 테마 삭제
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
/// * `theme_id` - 삭제할 테마의 ID
///
/// # Errors
/// * [`AppError::Validation`] - 테마 ID가 유효하지 않거나 기본 테마 ID인 경우
/// * [`AppError::Io`] - 파일 삭제 실패
#[tauri::command]
pub(crate) async fn delete_theme(
    app_handle: tauri::AppHandle,
    theme_id: String,
) -> Result<bool, AppError> {
    validate_theme_id(&theme_id)?;
    let themes_dir = get_themes_dir(&app_handle)?;
    let result = store::spawn_io(move || delete_custom_theme(&themes_dir, &theme_id)).await?;
    Ok(result)
}

fn get_themes_dir(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    let data_dir = store::get_data_dir(app_handle)?;
    Ok(data_dir.join(THEMES_DIR_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn list_themes_returns_two_defaults_when_dir_missing() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("nonexistent");
        let entries = list_themes(&themes_dir);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "default-dark");
        assert_eq!(entries[1].id, "default-light");
    }

    #[test]
    fn list_themes_returns_two_defaults_when_dir_empty() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        let entries = list_themes(&themes_dir);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn list_themes_includes_custom_theme_files() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(
            themes_dir.join("tokyo-night.json"),
            r#"{"name": "Tokyo Night", "type": "dark"}"#,
        )
        .unwrap();
        let entries = list_themes(&themes_dir);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].id, "tokyo-night");
        assert_eq!(entries[2].name, "Tokyo Night");
    }

    #[test]
    fn list_themes_skips_invalid_json_files() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(themes_dir.join("bad.json"), "not json").unwrap();
        let entries = list_themes(&themes_dir);
        assert_eq!(entries.len(), 2); // only defaults
    }

    #[test]
    fn list_themes_skips_non_json_files() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(themes_dir.join("readme.txt"), "not a theme").unwrap();
        let entries = list_themes(&themes_dir);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_theme_returns_default_dark() {
        let dir = TempDir::new().unwrap();
        let theme = load_theme(dir.path(), "default-dark").unwrap();
        assert_eq!(theme.name, "Dark Theme");
        assert!(matches!(theme.theme_type, ThemeType::Dark));
        assert!(!theme.colors.is_empty());
    }

    #[test]
    fn load_theme_returns_default_light() {
        let dir = TempDir::new().unwrap();
        let theme = load_theme(dir.path(), "default-light").unwrap();
        assert_eq!(theme.name, "Light Theme");
        assert!(matches!(theme.theme_type, ThemeType::Light));
    }

    #[test]
    fn load_theme_loads_custom_file() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("my-theme.json"),
            r##"{"name": "My Theme", "type": "dark", "colors": {"accent": "#ff0000"}}"##,
        )
        .unwrap();
        let theme = load_theme(dir.path(), "my-theme").unwrap();
        assert_eq!(theme.name, "My Theme");
        assert_eq!(theme.colors.get("accent").unwrap(), "#ff0000");
    }

    #[test]
    fn load_theme_returns_error_for_missing_file() {
        let dir = TempDir::new().unwrap();
        let result = load_theme(dir.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn load_theme_returns_error_for_invalid_json() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bad.json"), "not json").unwrap();
        let result = load_theme(dir.path(), "bad");
        assert!(result.is_err());
    }

    #[test]
    fn theme_list_entry_serializes_type_field() {
        let entry = ListEntry {
            id: "test".to_string(),
            name: "Test".to_string(),
            theme_type: ThemeType::Dark,
            description: "".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""type":"dark""#));
        assert!(!json.contains("theme_type"));
    }

    #[test]
    fn save_custom_theme_creates_json_file() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("themes");
        let theme = File {
            name: "My Theme".to_string(),
            description: "custom".to_string(),
            theme_type: ThemeType::Dark,
            colors: std::collections::HashMap::from([("accent".to_string(), "#ff0000".to_string())]),
        };
        let saved = save_custom_theme(&themes_dir, "my-theme", &theme).unwrap();
        assert_eq!(saved.name, "My Theme");
        assert!(themes_dir.join("my-theme.json").exists());
        let loaded = load_theme(&themes_dir, "my-theme").unwrap();
        assert_eq!(loaded.name, "My Theme");
    }

    #[test]
    fn save_custom_theme_rejects_default_ids() {
        let dir = TempDir::new().unwrap();
        let theme = default_dark_theme();
        assert!(save_custom_theme(dir.path(), "default-dark", &theme).is_err());
        assert!(save_custom_theme(dir.path(), "default-light", &theme).is_err());
    }

    #[test]
    fn delete_custom_theme_removes_file() {
        let dir = TempDir::new().unwrap();
        let themes_dir = dir.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(themes_dir.join("old.json"), r#"{"name":"Old","type":"dark"}"#).unwrap();
        let result = delete_custom_theme(&themes_dir, "old").unwrap();
        assert!(result);
        assert!(!themes_dir.join("old.json").exists());
    }

    #[test]
    fn delete_custom_theme_rejects_default_ids() {
        let dir = TempDir::new().unwrap();
        assert!(delete_custom_theme(dir.path(), "default-dark").is_err());
        assert!(delete_custom_theme(dir.path(), "default-light").is_err());
    }

    #[test]
    fn validate_theme_id_rejects_path_traversal() {
        assert!(validate_theme_id("../etc/passwd").is_err());
        assert!(validate_theme_id("..\\windows\\system32").is_err());
        assert!(validate_theme_id("foo/bar").is_err());
        assert!(validate_theme_id("foo\\bar").is_err());
        assert!(validate_theme_id("valid-theme").is_ok());
        assert!(validate_theme_id("my_theme.v2").is_ok());
    }
}
