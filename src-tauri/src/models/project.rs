use serde::{Deserialize, Serialize};

/// 프로젝트 정보를 담는 구조체. IPC를 통해 프론트엔드와 JSON으로 직렬화/역직렬화됨.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Info {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) created_at: String,
    #[serde(default)]
    pub(crate) sort_order: u32,
}

/// 프로젝트 목록을 담는 래퍼 구조체. projects.json 파일의 루트 형상에 대응.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Store {
    pub(crate) projects: Vec<Info>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_serializes_to_camel_case_json() {
        let project = Info {
            id: "test-id".to_string(),
            name: "My Project".to_string(),
            path: "C:\\projects\\test".to_string(),
            created_at: "2026-03-09T10:00:00Z".to_string(),
            sort_order: 0,
        };

        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("\"createdAt\""));
        assert!(!json.contains("\"created_at\""));
    }

    #[test]
    fn project_deserializes_from_camel_case_json() {
        let json = r#"{"id":"abc","name":"test","path":"C:\\test","createdAt":"2026-01-01T00:00:00Z"}"#;
        let project: Info = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "abc");
        assert_eq!(project.created_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn projects_data_wraps_project_vec() {
        let data = Store {
            projects: vec![
                Info {
                    id: "1".to_string(),
                    name: "P1".to_string(),
                    path: "/p1".to_string(),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    sort_order: 0,
                },
            ],
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"projects\""));
        assert!(json.contains("\"P1\""));
    }

    #[test]
    fn projects_data_deserializes_empty() {
        let json = r#"{"projects":[]}"#;
        let data: Store = serde_json::from_str(json).unwrap();
        assert!(data.projects.is_empty());
    }

    #[test]
    fn sort_order_defaults_to_zero_when_missing_in_json() {
        let json = r#"{"id":"abc","name":"test","path":"C:\\test","createdAt":"2026-01-01T00:00:00Z"}"#;
        let project: Info = serde_json::from_str(json).unwrap();
        assert_eq!(project.sort_order, 0);
    }

    #[test]
    fn sort_order_serializes_to_camel_case() {
        let project = Info {
            id: "test-id".to_string(),
            name: "Test".to_string(),
            path: "/test".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            sort_order: 5,
        };
        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains("\"sortOrder\":5"));
    }

    #[test]
    fn sort_order_round_trips_through_json() {
        let project = Info {
            id: "1".to_string(),
            name: "P".to_string(),
            path: "/p".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            sort_order: 3,
        };
        let json = serde_json::to_string(&project).unwrap();
        let loaded: Info = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.sort_order, 3);
    }
}
