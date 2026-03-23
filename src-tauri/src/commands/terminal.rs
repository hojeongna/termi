use std::sync::{Arc, Mutex};

use crate::error::AppError;
use crate::models::terminal::{ExternalTerminalInfo, Instance};
use crate::services::notifier;
use crate::services::process_manager::Manager as ProcessManager;
use crate::services::reminder::Reminder;
use crate::store;

use super::validation::{validate_id, validate_text_chars, find_project_by_id};

/// 터미널 이름의 최대 허용 길이
const MAX_TERMINAL_NAME_LENGTH: usize = 50;

/// 탭 타이틀의 최대 허용 길이
const MAX_TAB_TITLE_LENGTH: usize = 200;

/// 지정된 프로젝트에 대해 새 터미널 인스턴스를 실행한다.
/// 프로젝트 ID로 프로젝트를 조회하고, 경로 유효성 검증 후 wt.exe를 실행한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
/// - `app_handle`: Tauri 앱 핸들
/// - `project_id`: 터미널을 실행할 프로젝트의 ID
///
/// # Errors
/// - `AppError::Validation`: project_id가 비어있거나 너무 긴 경우, 프로젝트 경로가 유효하지 않은 경우
/// - `AppError::ProjectNotFound`: 해당 ID의 프로젝트가 존재하지 않는 경우
/// - `AppError::Io`: wt.exe 실행 실패 시
#[tauri::command]
pub(crate) async fn launch_terminal(
    state: tauri::State<'_, ProcessManager>,
    app_handle: tauri::AppHandle,
    project_id: String,
) -> Result<Instance, AppError> {
    validate_id(&project_id, "project_id")?;

    let data_dir = store::get_data_dir(&app_handle)?;
    let project_id_clone = project_id.clone();
    let data_dir_clone = data_dir.clone();

    let (project, settings) = store::spawn_io(move || {
        let projects_data = store::load_projects_from_path(&data_dir_clone)?;
        let project = find_project_by_id(&projects_data.projects, &project_id_clone)?;

        match std::fs::metadata(&project.path) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                return Err(AppError::Validation(
                    format!("프로젝트 경로가 디렉터리가 아닙니다: {}", project.path)
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(AppError::Validation(
                    format!("프로젝트 경로를 찾을 수 없습니다: {}", project.path)
                ));
            }
            Err(e) => {
                return Err(AppError::Io(e));
            }
        }

        let settings = store::load_settings(&data_dir_clone);
        Ok((project, settings))
    })
    .await?;

    // 실행 가능한 프로세스의 허용 목록(allowlist) 검증은 ProcessManager::launch에 위임된다.
    state.launch(&project.id, &project.name, &project.path, &settings.language)
}

/// 현재 등록된 모든 터미널 인스턴스 목록을 반환한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
///
/// # Errors
/// - `AppError::Validation`: 내부 잠금(lock) 실패 시
#[tauri::command]
pub(crate) async fn get_terminals(
    state: tauri::State<'_, ProcessManager>,
) -> Result<Vec<Instance>, AppError> {
    state.get_all()
}

/// 지정된 터미널의 Windows Terminal 창을 포그라운드로 이동한다.
/// 포커스 시 해당 터미널의 미확인 알림을 자동 확인(acknowledge) 처리하고 리마인더도 중지한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
/// - `notifier`: 알림 서비스 상태
/// - `reminder`: 리마인더 서비스 상태
/// - `terminal_id`: 포커스할 터미널의 ID
///
/// # Errors
/// - `AppError::Validation`: terminal_id가 비어있거나 너무 긴 경우
/// - `AppError::Terminal`: 터미널을 찾을 수 없거나 창이 닫힌 경우
/// - notifier/reminder lock 실패 시 로그 출력 후 계속 진행 (에러 미전파)
#[tauri::command]
pub(crate) async fn focus_terminal(
    state: tauri::State<'_, ProcessManager>,
    notifier: tauri::State<'_, Arc<Mutex<notifier::Notifier>>>,
    reminder: tauri::State<'_, Arc<Mutex<Reminder>>>,
    terminal_id: String,
) -> Result<Instance, AppError> {
    validate_id(&terminal_id, "terminal_id")?;

    let instance = state.focus(&terminal_id)?;

    match notifier.lock() {
        Ok(mut n) => n.acknowledge(&terminal_id),
        Err(e) => eprintln!("notifier lock poisoned in focus_terminal: {}", e),
    }
    match reminder.lock() {
        Ok(mut r) => r.stop_reminder(&terminal_id),
        Err(e) => eprintln!("reminder lock poisoned in focus_terminal: {}", e),
    }

    Ok(instance)
}

/// 지정된 터미널을 목록에서 제거하고 종료 이벤트를 발행한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
/// - `app_handle`: Tauri 앱 핸들 (이벤트 발행용)
/// - `terminal_id`: 닫을 터미널의 ID
///
/// # Errors
/// - `AppError::Validation`: terminal_id가 비어있거나 너무 긴 경우
/// - `AppError::Terminal`: 터미널을 찾을 수 없는 경우
#[tauri::command]
pub(crate) async fn close_terminal(
    state: tauri::State<'_, ProcessManager>,
    app_handle: tauri::AppHandle,
    terminal_id: String,
) -> Result<Vec<crate::models::terminal::Instance>, AppError> {
    validate_id(&terminal_id, "terminal_id")?;

    state.close(&terminal_id, &app_handle)
}

/// 터미널의 이름을 변경한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
/// - `terminal_id`: 이름을 변경할 터미널의 ID
/// - `new_name`: 새 터미널 이름 (1~50자)
///
/// # Errors
/// - `AppError::Validation`: terminal_id가 비어있거나 너무 긴 경우, 이름이 비어있거나 50자 초과인 경우
/// - `AppError::Terminal`: 터미널을 찾을 수 없는 경우
#[tauri::command]
pub(crate) async fn rename_terminal(
    state: tauri::State<'_, ProcessManager>,
    terminal_id: String,
    new_name: String,
) -> Result<Instance, AppError> {
    validate_id(&terminal_id, "terminal_id")?;
    if new_name.is_empty() || new_name.len() > MAX_TERMINAL_NAME_LENGTH {
        return Err(AppError::Validation(format!("터미널 이름은 1~{}자여야 합니다", MAX_TERMINAL_NAME_LENGTH)));
    }
    validate_text_chars(&new_name, "터미널 이름")?;
    state.rename(&terminal_id, &new_name)
}

/// 터미널의 알림 활성화 상태를 토글한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
/// - `terminal_id`: 알림을 토글할 터미널의 ID
///
/// # Errors
/// - `AppError::Validation`: terminal_id가 비어있거나 너무 긴 경우
/// - `AppError::Terminal`: 터미널을 찾을 수 없는 경우
#[tauri::command]
pub(crate) async fn toggle_terminal_notification(
    state: tauri::State<'_, ProcessManager>,
    terminal_id: String,
) -> Result<Instance, AppError> {
    validate_id(&terminal_id, "terminal_id")?;
    state.toggle_notification(&terminal_id)
}

/// 알림을 확인(acknowledge) 처리하여 pending에서 제거하고 리마인더도 중지한다.
///
/// # Parameters
/// - `notifier`: 알림 서비스 상태
/// - `reminder`: 리마인더 서비스 상태
/// - `terminal_id`: 확인 처리할 알림의 터미널 ID
///
/// # Errors
/// - `AppError::Validation`: terminal_id가 비어있거나 너무 긴 경우
/// - `AppError::Notification`: notifier 잠금(lock) 실패 시
#[tauri::command]
pub(crate) async fn acknowledge_notification(
    notifier: tauri::State<'_, Arc<Mutex<notifier::Notifier>>>,
    reminder: tauri::State<'_, Arc<Mutex<Reminder>>>,
    terminal_id: String,
) -> Result<bool, AppError> {
    validate_id(&terminal_id, "terminal_id")?;

    let mut n = notifier.lock().map_err(|e| {
        AppError::Notification(format!("notifier 잠금 실패: {}", e))
    })?;
    n.acknowledge(&terminal_id);
    drop(n);

    if let Ok(mut r) = reminder.lock() {
        r.stop_reminder(&terminal_id);
    }
    Ok(true)
}

/// 현재 실행 중인 외부 Windows Terminal 윈도우를 탐색하여 목록을 반환한다.
/// Termi가 이미 관리 중인 HWND는 제외된다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
///
/// # Errors
/// - `AppError::Validation`: 내부 잠금(lock) 실패 시
/// - `AppError::Io`: 외부 터미널 탐색 중 I/O 에러 발생 시
#[tauri::command]
pub(crate) async fn discover_external_terminals(
    state: tauri::State<'_, ProcessManager>,
) -> Result<Vec<ExternalTerminalInfo>, AppError> {
    state.discover_external_terminals()
}

/// 외부 터미널 탭을 지정된 프로젝트에 어태치하여 Termi 관리 목록에 등록한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
/// - `app_handle`: Tauri 앱 핸들
/// - `hwnd`: 어태치할 윈도우의 핸들 (0이 아닌 유효한 값이어야 함)
/// - `runtime_id`: UIA 런타임 ID (비어 있지 않아야 함)
/// - `tab_title`: 탭 타이틀 (1~200자)
/// - `project_id`: 연결할 프로젝트의 ID
///
/// # Errors
/// - `AppError::Validation`: hwnd가 0이거나, runtime_id가 비어있거나, tab_title이 비어있거나 200자 초과인 경우, project_id가 유효하지 않은 경우
/// - `AppError::ProjectNotFound`: 해당 ID의 프로젝트가 존재하지 않는 경우
#[tauri::command]
pub(crate) async fn attach_terminal(
    state: tauri::State<'_, ProcessManager>,
    app_handle: tauri::AppHandle,
    hwnd: isize,
    runtime_id: Vec<i32>,
    tab_title: String,
    project_id: String,
) -> Result<Instance, AppError> {
    validate_id(&project_id, "project_id")?;
    if hwnd == 0 {
        return Err(AppError::Validation("hwnd는 0일 수 없습니다".to_string()));
    }
    if runtime_id.is_empty() {
        return Err(AppError::Validation("runtime_id는 비어 있을 수 없습니다".to_string()));
    }
    if tab_title.is_empty() || tab_title.len() > MAX_TAB_TITLE_LENGTH {
        return Err(AppError::Validation(format!("탭 타이틀은 1~{}자여야 합니다", MAX_TAB_TITLE_LENGTH)));
    }
    validate_text_chars(&tab_title, "탭 타이틀")?;

    let data_dir = store::get_data_dir(&app_handle)?;
    let project_id_clone = project_id.clone();
    let data_dir_clone = data_dir.clone();

    let (project, settings) = store::spawn_io(move || {
        let projects_data = store::load_projects_from_path(&data_dir_clone)?;
        let project = find_project_by_id(&projects_data.projects, &project_id_clone)?;

        let settings = store::load_settings(&data_dir_clone);
        Ok((project, settings))
    })
    .await?;

    state.attach_terminal(
        hwnd,
        runtime_id,
        &tab_title,
        &project.id,
        &project.name,
        &project.path,
        &settings.language,
    )
}

/// 외부 WT 윈도우의 탭을 Termi WT 윈도우로 물리적으로 이동(가져오기)한다.
/// WT moveTab 액션 + 키바인딩 주입 + SendInput 시뮬레이션을 사용한다.
///
/// # Parameters
/// - `state`: ProcessManager 상태
/// - `app_handle`: Tauri 앱 핸들
/// - `hwnd`: 소스 윈도우 핸들
/// - `runtime_id`: UIA 런타임 ID
/// - `tab_title`: 이동할 탭의 제목
/// - `project_id`: 대상 프로젝트의 ID
///
/// # Errors
/// - `AppError::Validation`: hwnd가 0이거나, runtime_id가 비어있거나, tab_title이 유효하지 않은 경우
/// - `AppError::Terminal`: WT settings.json 탐지 실패, 탭 이동 실패
#[tauri::command]
pub(crate) async fn import_external_tab(
    state: tauri::State<'_, ProcessManager>,
    app_handle: tauri::AppHandle,
    hwnd: isize,
    runtime_id: Vec<i32>,
    tab_title: String,
    project_id: String,
) -> Result<Instance, AppError> {
    validate_id(&project_id, "project_id")?;
    if hwnd == 0 {
        return Err(AppError::Validation("hwnd는 0일 수 없습니다".to_string()));
    }
    if runtime_id.is_empty() {
        return Err(AppError::Validation("runtime_id는 비어 있을 수 없습니다".to_string()));
    }
    if tab_title.is_empty() || tab_title.len() > MAX_TAB_TITLE_LENGTH {
        return Err(AppError::Validation(format!("탭 타이틀은 1~{}자여야 합니다", MAX_TAB_TITLE_LENGTH)));
    }
    validate_text_chars(&tab_title, "탭 타이틀")?;

    let data_dir = store::get_data_dir(&app_handle)?;
    let project_id_clone = project_id.clone();
    let data_dir_clone = data_dir.clone();

    let (project, settings) = store::spawn_io(move || {
        let projects_data = store::load_projects_from_path(&data_dir_clone)?;
        let project = find_project_by_id(&projects_data.projects, &project_id_clone)?;
        let settings = store::load_settings(&data_dir_clone);
        Ok((project, settings))
    })
    .await?;

    state.attach_terminal(
        hwnd,
        runtime_id,
        &tab_title,
        &project.id,
        &project.name,
        &project.path,
        &settings.language,
    )
}
