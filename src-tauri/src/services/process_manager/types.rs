use crate::models::terminal::{Instance, Activity};

/// 허용된 터미널 실행 파일 목록
pub(super) const ALLOWED_TERMINAL_EXE: &str = "wt.exe";

/// Windows Terminal의 윈도우 클래스명
pub(super) const WT_WINDOW_CLASS: &str = "CASCADIA_HOSTING_WINDOW_CLASS";

/// 터미널 탭 제목 prefix — 이 문자열로 시작하면 리네임되지 않은 탭
pub(super) const TITLE_PREFIX: &str = "termi: ";

/// HWND 탐색 시작 전 초기 대기 시간 (밀리초)
pub(super) const HWND_INITIAL_DELAY_MS: u64 = 500;

/// HWND 탐색 최대 재시도 횟수
pub(super) const HWND_SEARCH_MAX_RETRIES: usize = 15;

/// HWND 탐색 재시도 간격 (밀리초)
pub(super) const HWND_SEARCH_INTERVAL_MS: u64 = 200;

/// 모니터링 폴링 간격 (초)
pub(super) const MONITOR_POLL_INTERVAL_SECS: u64 = 1;

/// 윈도우 제목에 이 문자열(✳)이 포함되면 Idle, 없으면 Active
pub(super) const TITLE_IDLE_MARKER: &str = "\u{2733}";

/// Idle→Active 전환 디바운스 시간 (초). ✳가 이 시간 이상 사라져야 Active 확정
pub(super) const ACTIVE_DEBOUNCE_SECS: u64 = 3;

/// 탭 포커스 후 안정화 대기 시간 (밀리초)
pub(super) const TAB_FOCUS_DELAY_MS: u64 = 300;

/// 상태 로그 출력 주기 (폴링 횟수 단위)
pub(super) const LOG_THROTTLE_POLL_INTERVAL: u64 = 5;

/// UIA matcher 검색 타임아웃 (밀리초)
pub(super) const UIA_MATCHER_TIMEOUT_MS: u64 = 1000;

/// UIA matcher 검색 최대 깊이
pub(super) const UIA_SEARCH_DEPTH: u32 = 20;

/// 'W' 키의 가상 키 코드
pub(super) const VK_W: u16 = 0x57;

/// F24 키의 가상 키 코드 (물리 키보드에 없는 가상 키)
pub(super) const VK_F24: u16 = 0x87;

/// 이름 필드 최대 길이 (commands::validation::MAX_NAME_LENGTH와 일치)
pub(super) const MAX_NAME_LENGTH: usize = 255;

/// PEB 내 ProcessParameters 포인터 오프셋 (x64)
pub(super) const PEB_PROCESS_PARAMETERS_OFFSET: usize = 0x20;

/// RTL_USER_PROCESS_PARAMETERS 내 CurrentDirectory 오프셋 (x64)
pub(super) const PROCESS_PARAMS_CWD_OFFSET: usize = 0x38;

/// UNICODE_STRING 내 Buffer 포인터 오프셋 (Length u16 + MaxLength u16 + padding u32)
pub(super) const UNICODE_STRING_BUFFER_PTR_OFFSET: usize = 8;

/// 첫 번째 탭의 인덱스
pub(super) const FIRST_TAB_INDEX: usize = 0;

/// 터미널 종료 이벤트명
pub(super) use crate::events::TERMINAL_CLOSED as EVENT_TERMINAL_CLOSED;

/// 터미널 순서 변경 이벤트명
pub(super) use crate::events::TERMINAL_ORDER_CHANGED as EVENT_TERMINAL_ORDER_CHANGED;

/// Termi가 관리하는 개별 터미널의 런타임 상태를 추적하는 구조체
pub(super) struct ManagedTerminal {
    pub(super) instance: Instance,
    pub(super) hwnd: Option<isize>,
    pub(super) tab_index: usize,
    /// UIA RuntimeId — 탭을 폴링 간 추적하는 고유 식별자
    pub(super) runtime_id: Option<Vec<i32>>,
    /// ✳가 한 번이라도 제목에 나타났으면 true → 이후 제목 기반 감지 활성화
    pub(super) title_tracking: bool,
    /// Idle→Active 전환 디바운스: ✳가 사라진 시점. 일정 시간 유지돼야 Active 확정
    pub(super) pending_active_since: Option<std::time::Instant>,
    /// wt.exe 프로세스 핸들 — 종료 시 fallback kill에 사용
    pub(super) child: Option<std::process::Child>,
    /// Claude Code session_id — hook 이벤트를 특정 터미널에 1:1 매핑
    pub(super) hook_session_id: Option<String>,
    /// 사용자가 탭 이름을 변경했는지 여부 — title에서 "termi: " prefix 소실로 감지
    pub(super) tab_renamed: bool,
    /// 외부 터미널을 어태치한 경우 true — 닫기 시 윈도우를 실제로 닫지 않음
    pub(super) attached: bool,
}

/// 셸 프로세스 실행 파일명 목록 (cwd를 읽을 대상)
pub(super) const SHELL_EXECUTABLES: &[&str] = &[
    "powershell.exe", "pwsh.exe", "cmd.exe", "bash.exe", "wsl.exe", "nu.exe", "fish.exe", "zsh.exe",
];

/// PROCESS_BASIC_INFORMATION (64-bit layout)
#[repr(C)]
pub(super) struct ProcessBasicInformation {
    pub(super) _exit_status: i32,
    pub(super) _pad0: u32,
    pub(super) peb_base_address: usize,
    pub(super) _affinity_mask: usize,
    pub(super) _base_priority: i32,
    pub(super) _pad1: u32,
    pub(super) _unique_process_id: usize,
    pub(super) _inherited_from: usize,
}

// NtQueryInformationProcess FFI (ntdll.dll)
#[link(name = "ntdll")]
unsafe extern "system" {
    pub(super) fn NtQueryInformationProcess(
        process_handle: windows_sys::Win32::Foundation::HANDLE,
        process_information_class: u32,
        process_information: *mut std::ffi::c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

/// Re-export `now_timestamp` from `crate::store` for convenience within this module.
/// Note: `debug_log::timestamp_millis()` uses millisecond precision for log entries —
/// different granularity, so the two functions are intentionally kept separate.
pub(super) use crate::store::now_timestamp;

/// 활동 전환 정보 튜플 타입 별칭
/// (terminal_id, project_path, project_name, terminal_name, new_activity, notification_enabled, monitored, last_idle_at)
pub(crate) type TransitionTuple = (String, String, String, String, Activity, bool, bool, Option<String>);

/// 다음 터미널 번호와 탭 인덱스를 계산한다.
/// launch()와 attach_terminal()에서 공통으로 사용.
pub(super) fn next_terminal_number_and_tab(
    terminals: &std::collections::HashMap<String, ManagedTerminal>,
    project_id: &str,
) -> (usize, usize) {
    let project_terminals: Vec<&ManagedTerminal> = terminals.values()
        .filter(|mt| mt.instance.project_id == project_id)
        .collect();
    let number = project_terminals.len() + 1;
    let next_tab = project_terminals.iter()
        .map(|mt| mt.tab_index)
        .max()
        .map(|max| max + 1)
        .unwrap_or(FIRST_TAB_INDEX);
    (number, next_tab)
}

/// 터미널 인스턴스를 생성한다.
/// launch()와 attach_terminal()에서 공통으로 사용.
pub(super) fn create_instance(
    project_id: &str,
    project_name: &str,
    project_path: &str,
    terminal_number: usize,
    language: &str,
    attached: bool,
) -> Instance {
    Instance {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        project_path: project_path.to_string(),
        terminal_name: match language {
            "ko" => format!("터미널 {}", terminal_number),
            _ => format!("Terminal {}", terminal_number),
        },
        status: crate::models::terminal::Status::Running,
        launched_at: now_timestamp(),
        activity: Activity::Active,
        notification_enabled: true,
        monitored: false,
        attached,
        last_idle_at: None,
    }
}
