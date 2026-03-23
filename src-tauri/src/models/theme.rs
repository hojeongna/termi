use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Represents a theme file with name, description, type, and color mappings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct File {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(rename = "type")]
    pub(crate) theme_type: ThemeType,
    #[serde(default)]
    pub(crate) colors: HashMap<String, String>,
}

/// Theme variant: dark or light.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ThemeType {
    Dark,
    Light,
}

impl Default for File {
    fn default() -> Self {
        default_dark_theme()
    }
}

/// Returns the built-in dark theme with predefined color mappings.
pub(crate) fn default_dark_theme() -> File {
    File {
        name: "Dark Theme".to_string(),
        description: "기본 다크 테마".to_string(),
        theme_type: ThemeType::Dark,
        colors: HashMap::from([
            ("bg-primary".into(), "#1e1e2e".into()),
            ("bg-secondary".into(), "#181825".into()),
            ("bg-surface".into(), "#313244".into()),
            ("text-primary".into(), "#cdd6f4".into()),
            ("text-secondary".into(), "#a6adc8".into()),
            ("accent".into(), "#89b4fa".into()),
            ("accent-hover".into(), "#74c7ec".into()),
            ("border".into(), "#45475a".into()),
            ("danger".into(), "#f38ba8".into()),
            ("success".into(), "#a6e3a1".into()),
            ("status-working".into(), "#22c55e".into()),
            ("status-waiting".into(), "#eab308".into()),
            ("status-completed".into(), "#ef4444".into()),
        ]),
    }
}

/// Returns the built-in light theme with predefined color mappings.
pub(crate) fn default_light_theme() -> File {
    File {
        name: "Light Theme".to_string(),
        description: "기본 라이트 테마".to_string(),
        theme_type: ThemeType::Light,
        colors: HashMap::from([
            ("bg-primary".into(), "#eff1f5".into()),
            ("bg-secondary".into(), "#e6e9ef".into()),
            ("bg-surface".into(), "#ccd0da".into()),
            ("text-primary".into(), "#4c4f69".into()),
            ("text-secondary".into(), "#6c6f85".into()),
            ("accent".into(), "#1e66f5".into()),
            ("accent-hover".into(), "#209fb5".into()),
            ("border".into(), "#bcc0cc".into()),
            ("danger".into(), "#d20f39".into()),
            ("success".into(), "#40a02b".into()),
            ("status-working".into(), "#40a02b".into()),
            ("status-waiting".into(), "#df8e1d".into()),
            ("status-completed".into(), "#d20f39".into()),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_file_deserializes_from_full_json() {
        let json = r##"{
            "name": "My Custom Dark",
            "description": "A custom dark theme",
            "type": "dark",
            "colors": {
                "bg-primary": "#1a1b26",
                "accent": "#7aa2f7"
            }
        }"##;
        let theme: File = serde_json::from_str(json).unwrap();
        assert_eq!(theme.name, "My Custom Dark");
        assert_eq!(theme.description, "A custom dark theme");
        assert!(matches!(theme.theme_type, ThemeType::Dark));
        assert_eq!(theme.colors.get("bg-primary").unwrap(), "#1a1b26");
        assert_eq!(theme.colors.get("accent").unwrap(), "#7aa2f7");
    }

    #[test]
    fn theme_file_deserializes_light_type() {
        let json = r#"{"name": "Light", "type": "light"}"#;
        let theme: File = serde_json::from_str(json).unwrap();
        assert!(matches!(theme.theme_type, ThemeType::Light));
    }

    #[test]
    fn theme_file_defaults_description_to_empty() {
        let json = r#"{"name": "Minimal", "type": "dark"}"#;
        let theme: File = serde_json::from_str(json).unwrap();
        assert_eq!(theme.description, "");
    }

    #[test]
    fn theme_file_defaults_colors_to_empty_map() {
        let json = r#"{"name": "Minimal", "type": "dark"}"#;
        let theme: File = serde_json::from_str(json).unwrap();
        assert!(theme.colors.is_empty());
    }

    #[test]
    fn theme_file_rejects_missing_name() {
        let json = r#"{"type": "dark"}"#;
        let result = serde_json::from_str::<File>(json);
        assert!(result.is_err());
    }

    #[test]
    fn theme_file_rejects_missing_type() {
        let json = r#"{"name": "No Type"}"#;
        let result = serde_json::from_str::<File>(json);
        assert!(result.is_err());
    }

    #[test]
    fn theme_file_rejects_invalid_type() {
        let json = r#"{"name": "Bad", "type": "invalid"}"#;
        let result = serde_json::from_str::<File>(json);
        assert!(result.is_err());
    }

    #[test]
    fn theme_file_serializes_to_json() {
        let theme = File {
            name: "Test".to_string(),
            description: "desc".to_string(),
            theme_type: ThemeType::Dark,
            colors: HashMap::from([("accent".to_string(), "#ff0000".to_string())]),
        };
        let json = serde_json::to_string(&theme).unwrap();
        assert!(json.contains(r#""name":"Test""#));
        assert!(json.contains(r#""type":"dark""#));
        assert!(json.contains("accent"));
    }

    #[test]
    fn theme_type_serializes_lowercase() {
        let dark_json = serde_json::to_string(&ThemeType::Dark).unwrap();
        let light_json = serde_json::to_string(&ThemeType::Light).unwrap();
        assert_eq!(dark_json, r#""dark""#);
        assert_eq!(light_json, r#""light""#);
    }

    #[test]
    fn default_dark_theme_has_correct_name_and_type() {
        let theme = default_dark_theme();
        assert_eq!(theme.name, "Dark Theme");
        assert!(matches!(theme.theme_type, ThemeType::Dark));
    }

    #[test]
    fn default_dark_theme_has_all_color_keys() {
        let theme = default_dark_theme();
        let expected_keys = [
            "bg-primary", "bg-secondary", "bg-surface",
            "text-primary", "text-secondary",
            "accent", "accent-hover", "border",
            "danger", "success",
            "status-working", "status-waiting", "status-completed",
        ];
        for key in expected_keys {
            assert!(theme.colors.contains_key(key), "missing key: {}", key);
        }
    }

    #[test]
    fn default_light_theme_has_correct_name_and_type() {
        let theme = default_light_theme();
        assert_eq!(theme.name, "Light Theme");
        assert!(matches!(theme.theme_type, ThemeType::Light));
    }

    #[test]
    fn default_light_theme_has_all_color_keys() {
        let theme = default_light_theme();
        let expected_keys = [
            "bg-primary", "bg-secondary", "bg-surface",
            "text-primary", "text-secondary",
            "accent", "accent-hover", "border",
            "danger", "success",
            "status-working", "status-waiting", "status-completed",
        ];
        for key in expected_keys {
            assert!(theme.colors.contains_key(key), "missing key: {}", key);
        }
    }

    #[test]
    fn default_dark_and_light_themes_have_different_colors() {
        let dark = default_dark_theme();
        let light = default_light_theme();
        assert_ne!(dark.colors.get("bg-primary"), light.colors.get("bg-primary"));
    }
}
