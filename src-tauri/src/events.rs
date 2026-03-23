/// Tauri event name constants for type-safe event emission and listening.
/// Frontend counterparts live in src/lib/constants.ts — keep both in sync.

/// 터미널 상태(Activity) 변경 이벤트
pub(crate) const TERMINAL_STATUS_CHANGED: &str = "terminal-status-changed";

/// 터미널 종료 이벤트
pub(crate) const TERMINAL_CLOSED: &str = "terminal-closed";

/// 외부 터미널 자동 어태치 이벤트
pub(crate) const TERMINAL_AUTO_ATTACHED: &str = "terminal-auto-attached";

/// 알림 클릭 이벤트
pub(crate) const NOTIFICATION_CLICKED: &str = "notification-clicked";

/// 디버그 로그 업데이트 이벤트
pub(crate) const DEBUG_LOG_UPDATED: &str = "debug-log-updated";

/// 터미널 탭 순서 변경 이벤트 (WT → Termi 단방향 동기화)
pub(crate) const TERMINAL_ORDER_CHANGED: &str = "terminal-order-changed";
