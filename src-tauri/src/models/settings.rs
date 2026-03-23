use serde::{Deserialize, Serialize};

const DEFAULT_REMINDER_INTERVAL_MINUTES: u32 = 5;
const DEFAULT_REMINDER_MAX_REPEAT: u32 = 3;
const DEFAULT_IDLE_THRESHOLD_SECS: u32 = 30;

/// 애플리케이션 전체 설정을 관리하는 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Config {
    /// 리마인더 알림의 동작 방식을 제어하는 중첩 설정 (활성화 여부, 간격, 반복 횟수)
    pub(crate) reminder: Reminder,
    /// 터미널이 유휴 상태로 간주되기까지의 시간(초). 이 시간 동안 입력이 없으면 idle로 표시된다.
    pub(crate) idle_threshold_secs: u32,
    /// 현재 활성화된 UI 테마의 식별자 (예: "default-dark", "default-light")
    pub(crate) theme: String,
    /// UI 표시 언어 코드 (예: "en", "ko"). i18n 번역 파일 선택에 사용된다.
    pub(crate) language: String,
    /// 앱 시작 시 외부 Windows Terminal 자동 탐색 및 어태치 활성화 여부
    pub(crate) auto_attach_enabled: bool,
    /// 윈도우를 항상 다른 윈도우 위에 표시할지 여부
    pub(crate) always_on_top: bool,
}

/// 리마인더 기능의 활성화 여부와 간격을 설정하는 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Reminder {
    /// 리마인더 알림 기능의 전체 활성화 여부. `false`이면 모든 리마인더 알림이 억제된다.
    pub(crate) enabled: bool,
    /// 리마인더 알림이 반복되는 간격(분). 마지막 알림으로부터 이 시간이 지나면 다시 알림을 표시한다.
    pub(crate) interval_minutes: u32,
    /// 동일 세션에서 리마인더 알림을 최대 몇 번까지 반복할지 상한 횟수. 이 횟수에 도달하면 알림이 중단된다.
    pub(crate) max_repeat: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            reminder: Reminder::default(),
            idle_threshold_secs: DEFAULT_IDLE_THRESHOLD_SECS,
            theme: "default-dark".to_string(),
            language: "en".to_string(),
            auto_attach_enabled: true,
            always_on_top: false,
        }
    }
}

impl Default for Reminder {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: DEFAULT_REMINDER_INTERVAL_MINUTES,
            max_repeat: DEFAULT_REMINDER_MAX_REPEAT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_has_reminder_enabled() {
        let settings = Config::default();
        assert!(settings.reminder.enabled);
    }

    #[test]
    fn default_settings_has_5_minute_interval() {
        let settings = Config::default();
        assert_eq!(settings.reminder.interval_minutes, 5);
    }

    #[test]
    fn default_settings_has_30_second_idle_threshold() {
        let settings = Config::default();
        assert_eq!(settings.idle_threshold_secs, 30);
    }

    #[test]
    fn settings_serializes_to_camel_case() {
        let settings = Config::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"intervalMinutes\""));
        assert!(json.contains("\"idleThresholdSecs\""));
        assert!(!json.contains("\"interval_minutes\""));
        assert!(!json.contains("\"idle_threshold_secs\""));
    }

    #[test]
    fn settings_deserializes_from_camel_case() {
        let json = r#"{"reminder":{"enabled":false,"intervalMinutes":10},"idleThresholdSecs":60}"#;
        let settings: Config = serde_json::from_str(json).unwrap();
        assert!(!settings.reminder.enabled);
        assert_eq!(settings.reminder.interval_minutes, 10);
        assert_eq!(settings.idle_threshold_secs, 60);
    }

    #[test]
    fn settings_deserializes_without_idle_threshold_uses_default() {
        let json = r#"{"reminder":{"enabled":true,"intervalMinutes":5}}"#;
        let settings: Config = serde_json::from_str(json).unwrap();
        assert_eq!(settings.idle_threshold_secs, 30);
    }

    #[test]
    fn settings_roundtrip() {
        let original = Config {
            reminder: Reminder {
                enabled: false,
                interval_minutes: 15,
                max_repeat: 5,
            },
            idle_threshold_secs: 45,
            theme: "custom-theme".to_string(),
            language: "en".to_string(),
            auto_attach_enabled: true,
            always_on_top: false,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.reminder.enabled, false);
        assert_eq!(deserialized.reminder.interval_minutes, 15);
        assert_eq!(deserialized.reminder.max_repeat, 5);
        assert_eq!(deserialized.idle_threshold_secs, 45);
        assert_eq!(deserialized.theme, "custom-theme");
    }

    #[test]
    fn default_settings_has_default_dark_theme() {
        let settings = Config::default();
        assert_eq!(settings.theme, "default-dark");
    }

    #[test]
    fn settings_deserializes_without_theme_uses_default() {
        let json = r#"{"reminder":{"enabled":true,"intervalMinutes":5}}"#;
        let settings: Config = serde_json::from_str(json).unwrap();
        assert_eq!(settings.theme, "default-dark");
    }

    #[test]
    fn default_settings_has_en_language() {
        let settings = Config::default();
        assert_eq!(settings.language, "en");
    }

    #[test]
    fn settings_deserializes_without_language_uses_default() {
        let json = r#"{"reminder":{"enabled":true,"intervalMinutes":5}}"#;
        let settings: Config = serde_json::from_str(json).unwrap();
        assert_eq!(settings.language, "en");
    }

    #[test]
    fn default_settings_has_auto_attach_enabled_true() {
        let settings = Config::default();
        assert!(settings.auto_attach_enabled);
    }

    #[test]
    fn settings_deserializes_without_auto_attach_uses_default_true() {
        let json = r#"{"reminder":{"enabled":true,"intervalMinutes":5}}"#;
        let settings: Config = serde_json::from_str(json).unwrap();
        assert!(settings.auto_attach_enabled);
    }

    #[test]
    fn settings_auto_attach_serializes_to_camel_case() {
        let mut settings = Config::default();
        settings.auto_attach_enabled = false;
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"autoAttachEnabled\":false"));
    }

    #[test]
    fn default_settings_has_always_on_top_false() {
        let settings = Config::default();
        assert!(!settings.always_on_top);
    }

    #[test]
    fn settings_deserializes_without_always_on_top_uses_default_false() {
        let json = r#"{"reminder":{"enabled":true,"intervalMinutes":5}}"#;
        let settings: Config = serde_json::from_str(json).unwrap();
        assert!(!settings.always_on_top);
    }

    #[test]
    fn settings_always_on_top_roundtrip() {
        let mut original = Config::default();
        original.always_on_top = true;
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert!(deserialized.always_on_top);
    }

    #[test]
    fn settings_always_on_top_serializes_to_camel_case() {
        let mut settings = Config::default();
        settings.always_on_top = true;
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"alwaysOnTop\":true"));
    }

    #[test]
    fn settings_roundtrip_with_language() {
        let original = Config {
            reminder: Reminder {
                enabled: true,
                interval_minutes: 5,
                max_repeat: 3,
            },
            idle_threshold_secs: 30,
            theme: "default-dark".to_string(),
            language: "ko".to_string(),
            auto_attach_enabled: true,
            always_on_top: false,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.language, "ko");
    }
}
