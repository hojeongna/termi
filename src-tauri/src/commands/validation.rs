use crate::error::AppError;
use crate::models::project;

/// ID 문자열의 최대 허용 길이
pub(crate) const MAX_ID_LENGTH: usize = 64;

/// 프로젝트 이름의 최대 허용 길이
pub(crate) const MAX_NAME_LENGTH: usize = 255;

/// 경로 문자열의 최대 허용 길이
pub(crate) const MAX_PATH_LENGTH: usize = 1024;

/// ID 문자열이 비어있지 않고 적절한 길이인지, 그리고 허용된 문자만 포함하는지 검증한다.
///
/// # Arguments
/// * `id` - 검증할 ID 문자열
/// * `field_name` - 에러 메시지에 표시할 필드 이름
///
/// # Errors
/// - 빈 문자열인 경우
/// - `MAX_ID_LENGTH`를 초과하는 경우
/// - 영숫자, '-', '_' 이외의 문자가 포함된 경우
pub(crate) fn validate_id(id: &str, field_name: &str) -> Result<(), AppError> {
    if id.is_empty() {
        return Err(AppError::Validation(format!("{}은(는) 비어 있을 수 없습니다", field_name)));
    }
    if id.len() > MAX_ID_LENGTH {
        return Err(AppError::Validation(format!("{}이(가) 너무 깁니다 (최대 {}자)", field_name, MAX_ID_LENGTH)));
    }
    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::Validation(format!(
            "{}에 허용되지 않는 문자가 포함되어 있습니다. 영숫자, '-', '_'만 사용할 수 있습니다",
            field_name
        )));
    }
    Ok(())
}

/// 이름 문자열을 검증하고 trim된 결과를 반환한다.
///
/// # Arguments
/// * `name` - 검증할 이름 문자열
/// * `field_name` - 에러 메시지에 표시할 필드 이름
/// * `max_length` - 최대 허용 길이
///
/// # Errors
/// - trim 후 빈 문자열인 경우
/// - `max_length`를 초과하는 경우
pub(crate) fn validate_name(name: &str, field_name: &str, max_length: usize) -> Result<String, AppError> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{}을(를) 입력해주세요", field_name)));
    }
    if trimmed.len() > max_length {
        return Err(AppError::Validation(format!(
            "{}은(는) {}자 이하여야 합니다",
            field_name, max_length
        )));
    }
    Ok(trimmed)
}

/// 경로 문자열을 검증하고 trim된 결과를 반환한다.
/// 디렉터리 존재 여부 확인은 I/O를 수반하므로 호출자에서 수행한다.
///
/// # Errors
/// - 빈 문자열인 경우
/// - `MAX_PATH_LENGTH`를 초과하는 경우
pub(crate) fn validate_path(path: &str) -> Result<String, AppError> {
    let trimmed = path.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Validation("경로를 입력해주세요".to_string()));
    }
    if trimmed.len() > MAX_PATH_LENGTH {
        return Err(AppError::Validation(format!(
            "경로는 {}자 이하여야 합니다",
            MAX_PATH_LENGTH
        )));
    }
    Ok(trimmed)
}

/// 표시용 텍스트(터미널 이름, 탭 타이틀 등)에 제어 문자가 포함되어 있지 않은지 검증한다.
///
/// # Arguments
/// * `text` - 검증할 텍스트 문자열
/// * `field_name` - 에러 메시지에 표시할 필드 이름
///
/// # Errors
/// - 제어 문자가 포함된 경우
pub(crate) fn validate_text_chars(text: &str, field_name: &str) -> Result<(), AppError> {
    if text.chars().any(|c| c.is_control()) {
        return Err(AppError::Validation(format!(
            "{}에 허용되지 않는 제어 문자가 포함되어 있습니다",
            field_name
        )));
    }
    Ok(())
}

/// 설정 문자열(테마, 언어 등)의 길이와 허용 문자를 검증한다.
///
/// # Arguments
/// * `value` - 검증할 문자열
/// * `field_name` - 에러 메시지에 표시할 필드 이름
/// * `max_length` - 최대 허용 길이
///
/// # Errors
/// - 빈 문자열인 경우
/// - `max_length`를 초과하는 경우
/// - 영숫자, '-', '_' 이외의 문자가 포함된 경우
pub(crate) fn validate_setting_string(value: &str, field_name: &str, max_length: usize) -> Result<(), AppError> {
    if value.is_empty() {
        return Err(AppError::Validation(format!("{}은(는) 비어 있을 수 없습니다", field_name)));
    }
    if value.len() > max_length {
        return Err(AppError::Validation(format!(
            "{}이(가) 너무 깁니다 (최대 {}자)", field_name, max_length
        )));
    }
    if !value.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::Validation(format!(
            "{}에 허용되지 않는 문자가 포함되어 있습니다. 영숫자, '-', '_'만 사용할 수 있습니다",
            field_name
        )));
    }
    Ok(())
}

/// 언어 코드가 지원되는 로캘인지 검증한다.
///
/// # Errors
/// - 지원되지 않는 언어 코드인 경우
pub(crate) fn validate_language(lang: &str) -> Result<(), AppError> {
    const SUPPORTED_LOCALES: &[&str] = &["en", "ko"];
    if !SUPPORTED_LOCALES.contains(&lang) {
        return Err(AppError::Validation(format!(
            "지원되지 않는 언어입니다: '{}'. 지원 언어: {}",
            lang,
            SUPPORTED_LOCALES.join(", ")
        )));
    }
    Ok(())
}

/// 프로젝트 목록에서 ID로 프로젝트를 찾아 반환한다.
///
/// # Arguments
/// * `projects` - 검색할 프로젝트 목록
/// * `id` - 찾을 프로젝트 ID
///
/// # Errors
/// - 해당 ID의 프로젝트가 없는 경우 `AppError::ProjectNotFound` 반환
pub(crate) fn find_project_by_id(projects: &[project::Info], id: &str) -> Result<project::Info, AppError> {
    projects.iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| AppError::ProjectNotFound(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- validate_id tests ---

    #[test]
    fn validate_id_accepts_valid_input() {
        assert!(validate_id("abc-123_XYZ", "id").is_ok());
    }

    #[test]
    fn validate_id_rejects_empty_input() {
        let err = validate_id("", "id").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn validate_id_rejects_too_long_input() {
        let long = "a".repeat(MAX_ID_LENGTH + 1);
        let err = validate_id(&long, "id").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn validate_id_rejects_invalid_characters() {
        let err = validate_id("bad id!", "id").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // --- validate_name tests ---

    #[test]
    fn validate_name_trims_and_accepts_valid_input() {
        let result = validate_name(" My Project ", "프로젝트 이름", 255).unwrap();
        assert_eq!(result, "My Project");
    }

    #[test]
    fn validate_name_rejects_empty_input() {
        let err = validate_name("   ", "프로젝트 이름", 255).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn validate_name_rejects_too_long_input() {
        let long = "a".repeat(256);
        let err = validate_name(&long, "프로젝트 이름", 255).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // --- validate_path tests ---

    #[test]
    fn validate_path_trims_and_accepts_valid_input() {
        let result = validate_path(" C:\\projects\\test ").unwrap();
        assert_eq!(result, "C:\\projects\\test");
    }

    #[test]
    fn validate_path_rejects_empty_input() {
        let err = validate_path("").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn validate_path_rejects_too_long_input() {
        let long = "a".repeat(MAX_PATH_LENGTH + 1);
        let err = validate_path(&long).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // --- find_project_by_id tests ---

    #[test]
    fn find_project_by_id_returns_matching_project() {
        let projects = vec![
            project::Info {
                id: "p1".to_string(),
                name: "Project 1".to_string(),
                path: "/path/1".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                sort_order: 0,
            },
        ];
        let result = find_project_by_id(&projects, "p1").unwrap();
        assert_eq!(result.name, "Project 1");
    }

    #[test]
    fn find_project_by_id_returns_error_for_missing_id() {
        let projects: Vec<project::Info> = vec![];
        let err = find_project_by_id(&projects, "nonexistent").unwrap_err();
        assert!(matches!(err, AppError::ProjectNotFound(_)));
    }
}
