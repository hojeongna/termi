use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::error::AppError;
use crate::models::settings::Config;
use crate::services::notifier::Notifier;
use crate::services::reminder::Reminder;
use crate::store;
use super::validation::{validate_setting_string, validate_language};

const MIN_INTERVAL_MINUTES: u32 = 1;
const MAX_INTERVAL_MINUTES: u32 = 60;
const MIN_IDLE_THRESHOLD_SECS: u32 = 5;
const MAX_IDLE_THRESHOLD_SECS: u32 = 120;
const MAX_REMINDER_REPEAT: u32 = 100;
const MAX_THEME_LENGTH: usize = 100;

/// 현재 설정을 반환한다. 파일 없으면 기본값 반환.
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
///
/// # Errors
/// 앱 데이터 디렉토리 경로를 가져올 수 없을 때 `AppError::Io` 반환
#[tauri::command]
pub(crate) async fn get_settings(app_handle: tauri::AppHandle) -> Result<Config, AppError> {
    let data_dir = store::get_data_dir(&app_handle)?;
    let settings = store::spawn_io(move || Ok(store::load_settings(&data_dir))).await?;
    Ok(settings)
}

/// 설정을 저장하고 Reminder 서비스에 즉시 반영한다.
///
/// # Parameters
/// * `app_handle` - Tauri 애플리케이션 핸들
/// * `settings` - 저장할 새 설정 값
///
/// # Errors
/// 유효성 검증 실패, 앱 데이터 디렉토리 경로 오류, 또는 파일 저장 실패 시 에러 반환
#[tauri::command]
pub(crate) async fn update_settings(
    app_handle: tauri::AppHandle,
    settings: Config,
) -> Result<Config, AppError> {
    if settings.reminder.interval_minutes < MIN_INTERVAL_MINUTES || settings.reminder.interval_minutes > MAX_INTERVAL_MINUTES {
        return Err(AppError::Validation(format!(
            "interval_minutes must be between {} and {}, got {}",
            MIN_INTERVAL_MINUTES, MAX_INTERVAL_MINUTES, settings.reminder.interval_minutes
        )));
    }
    if settings.idle_threshold_secs < MIN_IDLE_THRESHOLD_SECS || settings.idle_threshold_secs > MAX_IDLE_THRESHOLD_SECS {
        return Err(AppError::Validation(format!(
            "idle_threshold_secs must be between {} and {}, got {}",
            MIN_IDLE_THRESHOLD_SECS, MAX_IDLE_THRESHOLD_SECS, settings.idle_threshold_secs
        )));
    }
    if settings.reminder.max_repeat > MAX_REMINDER_REPEAT {
        return Err(AppError::Validation(format!(
            "max_repeat must be between 0 and {}, got {}",
            MAX_REMINDER_REPEAT, settings.reminder.max_repeat
        )));
    }
    validate_setting_string(&settings.theme, "theme", MAX_THEME_LENGTH)?;
    validate_language(&settings.language)?;

    let data_dir = store::get_data_dir(&app_handle)?;
    let settings_clone = settings.clone();
    store::spawn_io(move || store::save_settings(&data_dir, &settings_clone)).await?;

    // Notifier 서비스에 언어 설정 반영
    let notifier_arc = {
        let state = app_handle.state::<Arc<Mutex<Notifier>>>();
        Arc::clone(state.inner())
    };
    if let Ok(mut n) = notifier_arc.lock() {
        n.set_language(&settings.language);
    } else {
        eprintln!("notifier lock poisoned in update_settings");
    };

    // Reminder 서비스에 설정 변경 반영
    let reminder_arc = {
        let state = app_handle.state::<Arc<Mutex<Reminder>>>();
        Arc::clone(state.inner())
    };
    if let Ok(mut r) = reminder_arc.lock() {
        r.update_settings(settings.reminder.enabled, settings.reminder.interval_minutes, settings.reminder.max_repeat, &settings.language);
    } else {
        eprintln!("reminder lock poisoned in update_settings");
    };

    // Always on Top 설정 즉시 반영
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.set_always_on_top(settings.always_on_top);
    }

    Ok(settings)
}
