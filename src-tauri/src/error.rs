use thiserror::Error;

/// 애플리케이션 에러 타입. 모든 Tauri 커맨드에서 공유 사용.
#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("IO 에러: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 파싱 에러: {0}")]
    Json(#[from] serde_json::Error),
    #[error("프로젝트를 찾을 수 없음: {0}")]
    ProjectNotFound(String),
    #[error("{0}")]
    Validation(String),
    #[error("터미널 에러: {0}")]
    Terminal(String),
    #[error("알림 에러: {0}")]
    Notification(String),
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_displays_message() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let app_err: AppError = io_err.into();
        assert!(app_err.to_string().contains("file missing"));
    }

    #[test]
    fn json_error_displays_message() {
        let json_err: serde_json::Error = serde_json::from_str::<String>("invalid").unwrap_err();
        let app_err: AppError = json_err.into();
        assert!(app_err.to_string().contains("JSON"));
    }

    #[test]
    fn project_not_found_displays_id() {
        let app_err = AppError::ProjectNotFound("abc-123".to_string());
        assert!(app_err.to_string().contains("abc-123"));
    }

    #[test]
    fn app_error_serializes_as_string() {
        let app_err = AppError::ProjectNotFound("test-id".to_string());
        let serialized = serde_json::to_string(&app_err).unwrap();
        assert!(serialized.contains("test-id"));
    }
}
