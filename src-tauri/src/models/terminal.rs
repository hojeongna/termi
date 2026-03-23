use serde::{Deserialize, Serialize};

/// 프로젝트에 연결된 터미널 세션의 실행 정보를 나타낸다.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Instance {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) project_name: String,
    pub(crate) project_path: String,
    pub(crate) terminal_name: String,
    pub(crate) status: Status,
    pub(crate) launched_at: String,
    pub(crate) activity: Activity,
    #[serde(default = "default_notification_enabled")]
    pub(crate) notification_enabled: bool,
    /// ✳가 한 번이라도 감지되었는지 여부. false면 상태 뱃지를 표시하지 않는다.
    #[serde(default)]
    pub(crate) monitored: bool,
    /// 외부 터미널을 어태치한 경우 true. 닫기 동작 및 시각적 구분에 사용.
    #[serde(default)]
    pub(crate) attached: bool,
    /// 마지막으로 Idle 상태로 전환된 시점의 타임스탬프. 프론트엔드 시간 표시에 사용.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_idle_at: Option<String>,
}

const DEFAULT_NOTIFICATION_ENABLED: bool = true;

fn default_notification_enabled() -> bool {
    DEFAULT_NOTIFICATION_ENABLED
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            id: String::new(),
            project_id: String::new(),
            project_name: String::new(),
            project_path: String::new(),
            terminal_name: String::new(),
            status: Status::Running,
            launched_at: String::new(),
            activity: Activity::Active,
            notification_enabled: DEFAULT_NOTIFICATION_ENABLED,
            monitored: false,
            attached: false,
            last_idle_at: None,
        }
    }
}

/// 터미널 인스턴스의 생명주기 상태.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Status {
    Running,
    Stopped,
}

/// 터미널 프로세스의 IO 활동 상태 (2상태).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Activity {
    /// 활성 -- IO 활동 있음
    Active,
    /// 정적 -- IO 활동 없음
    Idle,
}

/// 외부 Windows Terminal 윈도우 탐색 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExternalTerminalInfo {
    pub(crate) hwnd: isize,
    pub(crate) window_title: String,
    pub(crate) tabs: Vec<TabInfo>,
    /// 윈도우 내 셸 프로세스들의 cwd 목록 (프로세스 트리에서 추출)
    #[serde(default)]
    pub(crate) process_cwds: Vec<String>,
}

/// Windows Terminal 탭 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TabInfo {
    pub(crate) runtime_id: Vec<i32>,
    pub(crate) title: String,
}

/// 프론트엔드로 전달되는 상태 변경 페이로드
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusChangedPayload {
    pub(crate) project_path: String,
    pub(crate) status: Activity,
    pub(crate) terminal_id: String,
    pub(crate) monitored: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_idle_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_terminal_info_serializes_to_camel_case() {
        let info = ExternalTerminalInfo {
            hwnd: 12345,
            window_title: "Windows Terminal".to_string(),
            tabs: vec![
                TabInfo {
                    runtime_id: vec![42, 1, 100],
                    title: "PowerShell".to_string(),
                },
            ],
            process_cwds: vec!["C:\\projects\\test".to_string()],
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"windowTitle\""));
        assert!(json.contains("\"runtimeId\""));
        assert!(!json.contains("\"window_title\""));
        assert!(!json.contains("\"runtime_id\""));
    }

    #[test]
    fn tab_info_serializes_to_camel_case() {
        let tab = TabInfo {
            runtime_id: vec![1, 2, 3],
            title: "bash".to_string(),
        };

        let json = serde_json::to_string(&tab).unwrap();
        assert!(json.contains("\"runtimeId\""));
        assert!(json.contains("\"title\""));
        assert!(!json.contains("\"runtime_id\""));
    }

    #[test]
    fn external_terminal_info_with_multiple_tabs() {
        let info = ExternalTerminalInfo {
            hwnd: 99999,
            window_title: "WT".to_string(),
            tabs: vec![
                TabInfo { runtime_id: vec![1], title: "Tab 1".to_string() },
                TabInfo { runtime_id: vec![2], title: "Tab 2".to_string() },
            ],
            process_cwds: vec![],
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ExternalTerminalInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.hwnd, 99999);
        assert_eq!(deserialized.tabs.len(), 2);
        assert_eq!(deserialized.tabs[0].title, "Tab 1");
        assert_eq!(deserialized.tabs[1].runtime_id, vec![2]);
    }

    #[test]
    fn external_terminal_info_with_empty_tabs() {
        let info = ExternalTerminalInfo {
            hwnd: 1,
            window_title: "WT".to_string(),
            tabs: vec![],
            process_cwds: vec![],
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: ExternalTerminalInfo = serde_json::from_str(&json).unwrap();
        assert!(deserialized.tabs.is_empty());
    }

    #[test]
    fn instance_attached_defaults_to_false() {
        let json = r#"{
            "id": "abc",
            "projectId": "proj-1",
            "projectName": "test",
            "projectPath": "C:\\test",
            "terminalName": "터미널 1",
            "status": "running",
            "launchedAt": "2026-01-01T00:00:00Z",
            "activity": "active"
        }"#;

        let instance: Instance = serde_json::from_str(json).unwrap();
        assert!(!instance.attached);
    }

    #[test]
    fn instance_attached_serializes_to_camel_case() {
        let mut instance = Instance::default();
        instance.attached = true;

        let json = serde_json::to_string(&instance).unwrap();
        assert!(json.contains("\"attached\":true"));
    }

    #[test]
    fn terminal_instance_serializes_to_camel_case() {
        let instance = Instance {
            id: "test-id".to_string(),
            project_id: "proj-1".to_string(),
            project_name: "my-project".to_string(),
            project_path: "C:\\projects\\my-project".to_string(),
            terminal_name: "터미널 1".to_string(),
            status: Status::Running,
            launched_at: "2026-03-09T10:00:00Z".to_string(),
            activity: Activity::Active,
            notification_enabled: true,
            monitored: false,
            attached: false,
            last_idle_at: None,
        };

        let json = serde_json::to_string(&instance).unwrap();
        assert!(json.contains("\"projectId\""));
        assert!(json.contains("\"projectName\""));
        assert!(json.contains("\"projectPath\""));
        assert!(json.contains("\"terminalName\""));
        assert!(json.contains("\"launchedAt\""));
        assert!(!json.contains("\"project_id\""));
    }

    #[test]
    fn terminal_instance_deserializes_from_camel_case() {
        let json = r#"{
            "id": "abc",
            "projectId": "proj-1",
            "projectName": "test",
            "projectPath": "C:\\test",
            "terminalName": "터미널 1",
            "status": "running",
            "launchedAt": "2026-01-01T00:00:00Z",
            "activity": "active"
        }"#;

        let instance: Instance = serde_json::from_str(json).unwrap();
        assert_eq!(instance.id, "abc");
        assert_eq!(instance.project_id, "proj-1");
        assert_eq!(instance.project_name, "test");
        assert_eq!(instance.status, Status::Running);
        assert_eq!(instance.activity, Activity::Active);
    }

    #[test]
    fn terminal_status_serializes_as_lowercase() {
        let running = serde_json::to_string(&Status::Running).unwrap();
        let stopped = serde_json::to_string(&Status::Stopped).unwrap();
        assert_eq!(running, "\"running\"");
        assert_eq!(stopped, "\"stopped\"");
    }

    #[test]
    fn terminal_status_deserializes_from_lowercase() {
        let running: Status = serde_json::from_str("\"running\"").unwrap();
        let stopped: Status = serde_json::from_str("\"stopped\"").unwrap();
        assert_eq!(running, Status::Running);
        assert_eq!(stopped, Status::Stopped);
    }

    #[test]
    fn terminal_activity_serializes_as_lowercase() {
        let active = serde_json::to_string(&Activity::Active).unwrap();
        let idle = serde_json::to_string(&Activity::Idle).unwrap();
        assert_eq!(active, "\"active\"");
        assert_eq!(idle, "\"idle\"");
    }

    #[test]
    fn terminal_activity_deserializes_from_lowercase() {
        let active: Activity = serde_json::from_str("\"active\"").unwrap();
        let idle: Activity = serde_json::from_str("\"idle\"").unwrap();
        assert_eq!(active, Activity::Active);
        assert_eq!(idle, Activity::Idle);
    }

    #[test]
    fn terminal_instance_serializes_activity() {
        let instance = Instance {
            id: "t1".to_string(),
            project_id: "p1".to_string(),
            project_name: "test".to_string(),
            project_path: "C:\\test".to_string(),
            terminal_name: "터미널 1".to_string(),
            status: Status::Running,
            launched_at: "2026-01-01T00:00:00Z".to_string(),
            activity: Activity::Idle,
            notification_enabled: true,
            monitored: false,
            attached: false,
            last_idle_at: None,
        };

        let json = serde_json::to_string(&instance).unwrap();
        assert!(json.contains("\"activity\":\"idle\""));
    }

    #[test]
    fn terminal_instance_roundtrip() {
        let original = Instance {
            id: "uuid-123".to_string(),
            project_id: "proj-456".to_string(),
            project_name: "테스트 프로젝트".to_string(),
            project_path: "C:\\projects\\test".to_string(),
            terminal_name: "터미널 1".to_string(),
            status: Status::Running,
            launched_at: "2026-03-09T10:00:00Z".to_string(),
            activity: Activity::Active,
            notification_enabled: true,
            monitored: false,
            attached: false,
            last_idle_at: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Instance = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, original.id);
        assert_eq!(deserialized.project_id, original.project_id);
        assert_eq!(deserialized.project_name, original.project_name);
        assert_eq!(deserialized.status, original.status);
        assert_eq!(deserialized.activity, original.activity);
    }

    #[test]
    fn notification_enabled_defaults_to_true_when_missing() {
        let json = r#"{
            "id": "abc",
            "projectId": "proj-1",
            "projectName": "test",
            "projectPath": "C:\\test",
            "terminalName": "터미널 1",
            "status": "running",
            "launchedAt": "2026-01-01T00:00:00Z",
            "activity": "active"
        }"#;

        let instance: Instance = serde_json::from_str(json).unwrap();
        assert!(instance.notification_enabled);
    }

    #[test]
    fn notification_enabled_serializes_to_camel_case() {
        let instance = Instance {
            id: "t1".to_string(),
            project_id: "p1".to_string(),
            project_name: "test".to_string(),
            project_path: "C:\\test".to_string(),
            terminal_name: "터미널 1".to_string(),
            status: Status::Running,
            launched_at: "2026-01-01T00:00:00Z".to_string(),
            activity: Activity::Active,
            notification_enabled: false,
            monitored: false,
            attached: false,
            last_idle_at: None,
        };

        let json = serde_json::to_string(&instance).unwrap();
        assert!(json.contains("\"notificationEnabled\":false"));
    }

    #[test]
    fn instance_last_idle_at_defaults_to_none() {
        let json = r#"{
            "id": "abc",
            "projectId": "proj-1",
            "projectName": "test",
            "projectPath": "C:\\test",
            "terminalName": "터미널 1",
            "status": "running",
            "launchedAt": "2026-01-01T00:00:00Z",
            "activity": "active"
        }"#;
        let instance: Instance = serde_json::from_str(json).unwrap();
        assert!(instance.last_idle_at.is_none());
    }

    #[test]
    fn instance_last_idle_at_roundtrip() {
        let mut instance = Instance::default();
        instance.last_idle_at = Some("1710500000Z".to_string());
        let json = serde_json::to_string(&instance).unwrap();
        assert!(json.contains("\"lastIdleAt\":\"1710500000Z\""));
        let deserialized: Instance = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.last_idle_at, Some("1710500000Z".to_string()));
    }

    #[test]
    fn instance_last_idle_at_none_skips_serialization() {
        let instance = Instance::default();
        let json = serde_json::to_string(&instance).unwrap();
        assert!(!json.contains("lastIdleAt"));
    }

    #[test]
    fn status_changed_payload_includes_last_idle_at() {
        let payload = StatusChangedPayload {
            project_path: "C:\\test".to_string(),
            status: Activity::Idle,
            terminal_id: "t1".to_string(),
            monitored: true,
            last_idle_at: Some("1710500000Z".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"lastIdleAt\":\"1710500000Z\""));
    }

    #[test]
    fn status_changed_payload_serializes_to_camel_case() {
        let payload = StatusChangedPayload {
            project_path: "C:\\projects\\test".to_string(),
            status: Activity::Idle,
            terminal_id: "t1".to_string(),
            monitored: true,
            last_idle_at: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"projectPath\""));
        assert!(json.contains("\"terminalId\""));
        assert!(json.contains("\"status\":\"idle\""));
        assert!(!json.contains("\"project_path\""));
    }
}
