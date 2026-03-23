use std::sync::Arc;

use tauri::Manager;

use crate::error::AppError;
use crate::services::debug_log::{DebugLog, LogEntry};

/// 현재 저장된 디버그 로그 전체를 반환한다.
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
///
/// # Errors
/// 현재 실패하는 경우는 없으나, 향후 확장을 위해 `Result` 타입으로 반환한다.
#[tauri::command]
pub(crate) async fn get_debug_logs(app_handle: tauri::AppHandle) -> Result<Vec<LogEntry>, AppError> {
    let log = app_handle.state::<Arc<DebugLog>>();
    Ok(log.get_all())
}

/// 디버그 로그를 모두 지운다.
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
///
/// # Errors
/// 현재 실패하는 경우는 없으나, 향후 확장을 위해 `Result` 타입으로 반환한다.
#[tauri::command]
pub(crate) async fn clear_debug_logs(app_handle: tauri::AppHandle) -> Result<bool, AppError> {
    let log = app_handle.state::<Arc<DebugLog>>();
    log.clear();
    Ok(true)
}
