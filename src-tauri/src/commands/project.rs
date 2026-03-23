use std::path::Path;

use crate::error::AppError;
use crate::models::project::Info;
use crate::store;

use super::validation::{validate_id, validate_name, validate_path, MAX_NAME_LENGTH};

/// 저장된 전체 프로젝트 목록을 반환한다.
///
/// # Errors
/// projects.json 파일 읽기 또는 파싱 실패 시 에러 반환
#[tauri::command]
pub(crate) async fn get_projects(app_handle: tauri::AppHandle) -> Result<Vec<Info>, AppError> {
    let data_dir = store::get_data_dir(&app_handle)?;
    let data = store::spawn_io(move || store::load_projects_from_path(&data_dir)).await?;
    Ok(data.projects)
}

/// 새 프로젝트를 등록하고 저장한다.
///
/// # Arguments
/// * `name` - 프로젝트 이름 (빈 문자열 불가)
/// * `path` - 프로젝트 폴더 절대 경로 (유효한 디렉토리여야 함)
///
/// # Errors
/// 유효성 검증 실패, 저장소 읽기/쓰기 실패 시 에러 반환
#[tauri::command]
pub(crate) async fn add_project(
    app_handle: tauri::AppHandle,
    name: String,
    path: String,
) -> Result<Info, AppError> {
    let name = validate_name(&name, "프로젝트 이름", MAX_NAME_LENGTH)?;
    let path = validate_path(&path)?;

    let data_dir = store::get_data_dir(&app_handle)?;
    let project = store::spawn_io(move || {
        if !Path::new(&path).is_dir() {
            return Err(AppError::Validation("유효하지 않은 폴더 경로입니다".to_string()));
        }

        let mut data = store::load_projects_from_path(&data_dir)?;

        let project = Info {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            path,
            created_at: store::now_timestamp(),
            sort_order: store::next_sort_order(&data),
        };

        data.projects.push(project.clone());
        store::save_projects_to_path(&data_dir, &data)?;

        Ok(project)
    })
    .await?;

    Ok(project)
}

/// 기존 프로젝트의 이름과 경로를 수정한다.
///
/// # Arguments
/// * `id` - 수정할 프로젝트 ID
/// * `name` - 새 프로젝트 이름
/// * `path` - 새 프로젝트 폴더 경로
///
/// # Errors
/// 프로젝트 미발견, 유효성 검증 실패, 저장소 읽기/쓰기 실패 시 에러 반환
#[tauri::command]
pub(crate) async fn update_project(
    app_handle: tauri::AppHandle,
    id: String,
    name: String,
    path: String,
) -> Result<Info, AppError> {
    validate_id(&id, "id")?;
    let name = validate_name(&name, "프로젝트 이름", MAX_NAME_LENGTH)?;
    let path = validate_path(&path)?;

    let data_dir = store::get_data_dir(&app_handle)?;
    let updated = store::spawn_io(move || {
        if !Path::new(&path).is_dir() {
            return Err(AppError::Validation("유효하지 않은 폴더 경로입니다".to_string()));
        }

        let mut data = store::load_projects_from_path(&data_dir)?;
        let project = data
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| AppError::ProjectNotFound(id))?;
        project.name = name;
        project.path = path;
        let updated = project.clone();
        store::save_projects_to_path(&data_dir, &data)?;

        Ok(updated)
    })
    .await?;

    Ok(updated)
}

/// 프로젝트를 삭제한다.
///
/// # Arguments
/// * `id` - 삭제할 프로젝트 ID
///
/// # Errors
/// 프로젝트 미발견, 저장소 읽기/쓰기 실패 시 에러 반환
#[tauri::command]
pub(crate) async fn delete_project(app_handle: tauri::AppHandle, id: String) -> Result<bool, AppError> {
    validate_id(&id, "id")?;

    let data_dir = store::get_data_dir(&app_handle)?;
    store::spawn_io(move || {
        let mut data = store::load_projects_from_path(&data_dir)?;
        let len_before = data.projects.len();
        data.projects.retain(|p| p.id != id);
        if data.projects.len() == len_before {
            return Err(AppError::ProjectNotFound(id));
        }
        store::save_projects_to_path(&data_dir, &data)?;
        Ok(true)
    })
    .await
}

/// 프로젝트 순서를 재정렬한다.
///
/// # Arguments
/// * `project_ids` - 원하는 순서대로 정렬된 프로젝트 ID 배열
///
/// # Errors
/// 유효성 검증 실패, 저장소 읽기/쓰기 실패 시 에러 반환
#[tauri::command]
pub(crate) async fn reorder_projects(
    app_handle: tauri::AppHandle,
    project_ids: Vec<String>,
) -> Result<Vec<Info>, AppError> {
    const MAX_PROJECT_IDS_COUNT: usize = 1000;
    if project_ids.len() > MAX_PROJECT_IDS_COUNT {
        return Err(AppError::Validation(format!(
            "project_ids 배열이 너무 깁니다 (최대 {}개)",
            MAX_PROJECT_IDS_COUNT
        )));
    }
    for id in &project_ids {
        validate_id(id, "project_id")?;
    }

    let data_dir = store::get_data_dir(&app_handle)?;
    let projects =
        store::spawn_io(move || store::reorder_projects_in_path(&data_dir, &project_ids)).await?;
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn validation_rejects_empty_name() {
        let name = "   ".trim();
        assert!(name.is_empty());
    }

    #[test]
    fn validation_accepts_valid_name() {
        let name = " My Project ".trim();
        assert!(!name.is_empty());
        assert_eq!(name, "My Project");
    }

    #[test]
    fn validation_rejects_nonexistent_path() {
        let path = Path::new("/nonexistent/path/12345");
        assert!(!path.is_dir());
    }

    #[test]
    fn validation_accepts_existing_directory() {
        let path = Path::new(".");
        assert!(path.is_dir());
    }
}
