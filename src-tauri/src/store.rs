use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::Manager;

use crate::error::AppError;
use crate::models::project::Store;
use crate::models::settings::Config;

/// Returns the current UTC timestamp as epoch seconds with a `Z` suffix (e.g. `"1710500000Z"`).
///
/// Uses seconds precision for user-facing timestamps (project `created_at`,
/// terminal `launched_at` / `last_idle_at`).
pub(crate) fn now_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    format!("{}Z", duration.as_secs())
}

/// Runs a blocking closure on a dedicated thread and awaits its result.
///
/// This is a convenience wrapper around `tauri::async_runtime::spawn_blocking`
/// that eliminates the repetitive `.await` + error mapping boilerplate.
pub(crate) async fn spawn_io<F, T>(f: F) -> Result<T, AppError>
where
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
}

const PROJECTS_FILE: &str = "projects.json";
const SETTINGS_FILE: &str = "settings.json";

/// Writes data to a file atomically by first writing to a temporary file
/// and then renaming it to the target path, preventing partial writes.
pub(crate) fn atomic_write(path: &Path, data: &[u8]) -> Result<(), std::io::Error> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// 앱 데이터 디렉토리 초기화: 필요한 디렉토리와 기본 JSON 파일 생성
pub(crate) fn init_data_dir(data_dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(data_dir)?;

    let projects_path = data_dir.join(PROJECTS_FILE);
    if !projects_path.exists() {
        atomic_write(&projects_path, b"{\"projects\":[]}")?;
    }

    let settings_path = data_dir.join(SETTINGS_FILE);
    if !settings_path.exists() {
        atomic_write(&settings_path, b"{}")?;
    }

    // themes 디렉토리 생성
    let themes_dir = data_dir.join("themes");
    fs::create_dir_all(&themes_dir)?;

    Ok(())
}

/// Returns the application data directory path for the given Tauri app handle.
///
/// # Parameters
/// - `app_handle`: A reference to the Tauri `AppHandle` used to resolve the platform-specific data directory.
pub(crate) fn get_data_dir(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, AppError> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
}

/// projects.json에서 프로젝트 목록 읽기
///
/// `sort_order`가 모두 0인 기존 데이터의 경우, 배열 인덱스 순으로 자동 할당한다.
pub(crate) fn load_projects_from_path(data_dir: &Path) -> Result<Store, AppError> {
    let path = data_dir.join(PROJECTS_FILE);
    if !path.exists() {
        return Ok(Store { projects: vec![] });
    }
    let content = fs::read_to_string(&path)?;
    let mut data: Store = match serde_json::from_str(&content) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("[store] projects.json 파싱 실패, 기본값 사용: {}", e);
            return Ok(Store { projects: vec![] });
        }
    };

    // 기존 데이터 하위호환: sort_order가 모두 0이면 배열 인덱스 순으로 재할당
    if data.projects.len() > 1 && data.projects.iter().all(|p| p.sort_order == 0) {
        for (i, project) in data.projects.iter_mut().enumerate() {
            project.sort_order = i as u32;
        }
    }

    Ok(data)
}

/// 다음 프로젝트에 할당할 sort_order 값을 반환한다 (현재 최대값 + 1).
pub(crate) fn next_sort_order(data: &Store) -> u32 {
    data.projects
        .iter()
        .map(|p| p.sort_order)
        .max()
        .map_or(0, |max| max + 1)
}

/// 프로젝트 목록을 주어진 ID 순서로 재정렬하고 저장한다.
///
/// `project_ids`에 없는 프로젝트는 무시된다. 재정렬 후 `sort_order`를 0부터 순차 할당한다.
pub(crate) fn reorder_projects_in_path(
    data_dir: &Path,
    project_ids: &[String],
) -> Result<Vec<crate::models::project::Info>, AppError> {
    let mut data = load_projects_from_path(data_dir)?;

    // project_ids 순서대로 정렬
    let id_order: HashMap<&str, usize> = project_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    data.projects.sort_by_key(|p| id_order.get(p.id.as_str()).copied().unwrap_or(usize::MAX));

    // sort_order 재할당
    for (i, project) in data.projects.iter_mut().enumerate() {
        project.sort_order = i as u32;
    }

    save_projects_to_path(data_dir, &data)?;
    Ok(data.projects)
}

/// projects.json에 프로젝트 목록 원자적 쓰기
pub(crate) fn save_projects_to_path(data_dir: &Path, data: &Store) -> Result<(), AppError> {
    let path = data_dir.join(PROJECTS_FILE);
    let json = serde_json::to_string_pretty(data)?;
    atomic_write(&path, json.as_bytes())?;
    Ok(())
}

/// settings.json에서 설정 읽기 (파일 없거나 파싱 실패 시 기본값)
pub(crate) fn load_settings(data_dir: &Path) -> Config {
    let path = data_dir.join(SETTINGS_FILE);
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("[store] settings.json 파싱 실패, 기본값 사용: {}", e);
                Config::default()
            }
        },
        Err(_) => Config::default(),
    }
}

/// settings.json에 설정 원자적 쓰기
pub(crate) fn save_settings(data_dir: &Path, settings: &Config) -> Result<(), AppError> {
    let path = data_dir.join(SETTINGS_FILE);
    let json = serde_json::to_string_pretty(settings)?;
    atomic_write(&path, json.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::Info;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_creates_file_with_correct_content() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.json");

        atomic_write(&file_path, b"hello world").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn atomic_write_overwrites_existing_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.json");

        fs::write(&file_path, "old content").unwrap();
        atomic_write(&file_path, b"new content").unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn atomic_write_does_not_leave_tmp_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.json");

        atomic_write(&file_path, b"data").unwrap();

        let tmp_path = file_path.with_extension("tmp");
        assert!(!tmp_path.exists());
    }

    #[test]
    fn init_data_dir_creates_directories() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("termi");

        init_data_dir(&data_dir).unwrap();

        assert!(data_dir.exists());
    }

    #[test]
    fn init_data_dir_creates_default_json_files() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("termi");

        init_data_dir(&data_dir).unwrap();

        let projects = fs::read_to_string(data_dir.join("projects.json")).unwrap();
        assert_eq!(projects, "{\"projects\":[]}");

        let settings = fs::read_to_string(data_dir.join("settings.json")).unwrap();
        assert_eq!(settings, "{}");
    }

    #[test]
    fn init_data_dir_does_not_overwrite_existing_files() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("termi");
        fs::create_dir_all(&data_dir).unwrap();

        let projects_path = data_dir.join("projects.json");
        fs::write(&projects_path, "{\"projects\":[{\"id\":\"1\"}]}").unwrap();

        init_data_dir(&data_dir).unwrap();

        let content = fs::read_to_string(&projects_path).unwrap();
        assert_eq!(content, "{\"projects\":[{\"id\":\"1\"}]}");
    }

    #[test]
    fn load_projects_returns_empty_when_no_file() {
        let dir = TempDir::new().unwrap();
        let data = load_projects_from_path(dir.path()).unwrap();
        assert!(data.projects.is_empty());
    }

    #[test]
    fn load_projects_reads_existing_data() {
        let dir = TempDir::new().unwrap();
        let json = r#"{"projects":[{"id":"1","name":"test","path":"/p","createdAt":"2026-01-01T00:00:00Z"}]}"#;
        fs::write(dir.path().join("projects.json"), json).unwrap();

        let data = load_projects_from_path(dir.path()).unwrap();
        assert_eq!(data.projects.len(), 1);
        assert_eq!(data.projects[0].name, "test");
    }

    #[test]
    fn save_projects_writes_atomically() {
        let dir = TempDir::new().unwrap();
        let data = Store {
            projects: vec![Info {
                id: "abc".to_string(),
                name: "My Project".to_string(),
                path: "C:\\test".to_string(),
                created_at: "2026-03-09T10:00:00Z".to_string(),
                sort_order: 0,
            }],
        };

        save_projects_to_path(dir.path(), &data).unwrap();

        let content = fs::read_to_string(dir.path().join("projects.json")).unwrap();
        assert!(content.contains("My Project"));
        assert!(content.contains("\"createdAt\""));
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let original = Store {
            projects: vec![
                Info {
                    id: "1".to_string(),
                    name: "First".to_string(),
                    path: "/first".to_string(),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    sort_order: 0,
                },
                Info {
                    id: "2".to_string(),
                    name: "Second".to_string(),
                    path: "/second".to_string(),
                    created_at: "2026-02-01T00:00:00Z".to_string(),
                    sort_order: 1,
                },
            ],
        };

        save_projects_to_path(dir.path(), &original).unwrap();
        let loaded = load_projects_from_path(dir.path()).unwrap();

        assert_eq!(loaded.projects.len(), 2);
        assert_eq!(loaded.projects[0].name, "First");
        assert_eq!(loaded.projects[1].name, "Second");
    }

    #[test]
    fn reorder_projects_assigns_sort_order_by_id_sequence() {
        let dir = TempDir::new().unwrap();
        let data = Store {
            projects: vec![
                Info {
                    id: "a".to_string(),
                    name: "A".to_string(),
                    path: "/a".to_string(),
                    created_at: "2026-01-01".to_string(),
                    sort_order: 0,
                },
                Info {
                    id: "b".to_string(),
                    name: "B".to_string(),
                    path: "/b".to_string(),
                    created_at: "2026-01-02".to_string(),
                    sort_order: 1,
                },
                Info {
                    id: "c".to_string(),
                    name: "C".to_string(),
                    path: "/c".to_string(),
                    created_at: "2026-01-03".to_string(),
                    sort_order: 2,
                },
            ],
        };
        save_projects_to_path(dir.path(), &data).unwrap();

        // Reorder: c, a, b
        let ids = vec!["c".to_string(), "a".to_string(), "b".to_string()];
        let result = reorder_projects_in_path(dir.path(), &ids).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, "c");
        assert_eq!(result[0].sort_order, 0);
        assert_eq!(result[1].id, "a");
        assert_eq!(result[1].sort_order, 1);
        assert_eq!(result[2].id, "b");
        assert_eq!(result[2].sort_order, 2);

        // Verify persisted
        let loaded = load_projects_from_path(dir.path()).unwrap();
        let mut sorted = loaded.projects;
        sorted.sort_by_key(|p| p.sort_order);
        assert_eq!(sorted[0].id, "c");
        assert_eq!(sorted[1].id, "a");
        assert_eq!(sorted[2].id, "b");
    }

    #[test]
    fn reorder_projects_ignores_unknown_ids() {
        let dir = TempDir::new().unwrap();
        let data = Store {
            projects: vec![
                Info {
                    id: "a".to_string(),
                    name: "A".to_string(),
                    path: "/a".to_string(),
                    created_at: "2026-01-01".to_string(),
                    sort_order: 0,
                },
            ],
        };
        save_projects_to_path(dir.path(), &data).unwrap();

        let ids = vec!["unknown".to_string(), "a".to_string()];
        let result = reorder_projects_in_path(dir.path(), &ids).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "a");
        assert_eq!(result[0].sort_order, 0);
    }

    #[test]
    fn next_sort_order_returns_zero_for_empty_projects() {
        let data = Store { projects: vec![] };
        assert_eq!(super::next_sort_order(&data), 0);
    }

    #[test]
    fn next_sort_order_returns_max_plus_one() {
        let data = Store {
            projects: vec![
                Info {
                    id: "1".to_string(),
                    name: "A".to_string(),
                    path: "/a".to_string(),
                    created_at: "2026-01-01".to_string(),
                    sort_order: 2,
                },
                Info {
                    id: "2".to_string(),
                    name: "B".to_string(),
                    path: "/b".to_string(),
                    created_at: "2026-01-02".to_string(),
                    sort_order: 5,
                },
            ],
        };
        assert_eq!(super::next_sort_order(&data), 6);
    }

    #[test]
    fn load_projects_assigns_sequential_sort_order_when_all_zero() {
        let dir = TempDir::new().unwrap();
        let json = r#"{"projects":[
            {"id":"1","name":"A","path":"/a","createdAt":"2026-01-01"},
            {"id":"2","name":"B","path":"/b","createdAt":"2026-01-02"},
            {"id":"3","name":"C","path":"/c","createdAt":"2026-01-03"}
        ]}"#;
        fs::write(dir.path().join("projects.json"), json).unwrap();

        let data = load_projects_from_path(dir.path()).unwrap();
        assert_eq!(data.projects[0].sort_order, 0);
        assert_eq!(data.projects[1].sort_order, 1);
        assert_eq!(data.projects[2].sort_order, 2);
    }

    #[test]
    fn load_projects_preserves_existing_sort_order() {
        let dir = TempDir::new().unwrap();
        let json = r#"{"projects":[
            {"id":"1","name":"A","path":"/a","createdAt":"2026-01-01","sortOrder":5},
            {"id":"2","name":"B","path":"/b","createdAt":"2026-01-02","sortOrder":2}
        ]}"#;
        fs::write(dir.path().join("projects.json"), json).unwrap();

        let data = load_projects_from_path(dir.path()).unwrap();
        assert_eq!(data.projects[0].sort_order, 5);
        assert_eq!(data.projects[1].sort_order, 2);
    }

    // Settings tests

    #[test]
    fn load_settings_returns_default_when_no_file() {
        let dir = TempDir::new().unwrap();
        let settings = load_settings(dir.path());
        assert!(settings.reminder.enabled);
        assert_eq!(settings.reminder.interval_minutes, 5);
    }

    #[test]
    fn load_settings_returns_default_for_invalid_json() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("settings.json"), "not json").unwrap();
        let settings = load_settings(dir.path());
        assert!(settings.reminder.enabled);
    }

    #[test]
    fn load_settings_returns_default_for_empty_json() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("settings.json"), "{}").unwrap();
        let settings = load_settings(dir.path());
        // Empty JSON should fail to parse Config (missing reminder field) → default
        assert!(settings.reminder.enabled);
    }

    #[test]
    fn save_then_load_settings_roundtrip() {
        let dir = TempDir::new().unwrap();
        let original = Config {
            reminder: crate::models::settings::Reminder {
                enabled: false,
                interval_minutes: 10,
                max_repeat: 3,
            },
            idle_threshold_secs: 45,
            theme: "default-dark".to_string(),
            language: "en".to_string(),
            auto_attach_enabled: true,
            always_on_top: false,
        };
        save_settings(dir.path(), &original).unwrap();
        let loaded = load_settings(dir.path());
        assert_eq!(loaded.reminder.enabled, false);
        assert_eq!(loaded.reminder.interval_minutes, 10);
        assert_eq!(loaded.idle_threshold_secs, 45);
    }

    #[test]
    fn save_settings_writes_atomically() {
        let dir = TempDir::new().unwrap();
        save_settings(dir.path(), &Config::default()).unwrap();

        let tmp = dir.path().join("settings.tmp");
        assert!(!tmp.exists());
    }
}
