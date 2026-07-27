use serde::{Deserialize, Serialize};

/// Settings management cho CULI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub default_model: String,
    pub auto_save: bool,
    pub show_token_usage: bool,
    #[serde(default)]
    pub custom_settings: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "vi".into(),
            default_model: "gpt-4o".into(),
            auto_save: true,
            show_token_usage: true,
            custom_settings: std::collections::HashMap::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        Settings::default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
