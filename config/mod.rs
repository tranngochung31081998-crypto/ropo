pub mod settings;
pub mod profile;

pub use settings::*;
pub use profile::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cấu hình chính của CULI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub provider: ProviderConfig,
    pub agent: AgentConfig,
    pub storage: StorageConfig,
    pub ui: UiConfig,
    #[serde(default)]
    pub data_dir: Option<String>,
    #[serde(default)]
    pub sub_agent: SubAgentConfig,
    #[serde(default)]
    pub engineer_models: EngineerModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    pub enabled: bool,
    pub context_threshold_tokens: usize,
    pub auto_reflect: bool,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            context_threshold_tokens: 150_000,
            auto_reflect: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub primary: String,
    pub fallbacks: Vec<String>,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub max_tokens_per_call: u32,
    pub temperature: f32,
    pub context_window: u32,
    pub enable_memory: bool,
    pub enable_graph: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: PathBuf,
    pub memory_path: PathBuf,
    pub skills_path: PathBuf,
    pub plugins_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub language: String,
    pub show_token_usage: bool,
}

/// Model assignment per agent role (engineer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineerModelConfig {
    /// Model for code generation / implementation tasks
    pub coder:     String,
    /// Model for code review / security audit tasks  
    pub reviewer:  String,
    /// Model for architecture planning / analysis
    pub architect: String,
    /// Model for quick harness tasks (tools, memory, internal)
    /// Should always use harness layer (sixth/blackbox) — NOT Qveris
    pub harness:   String,
}

impl Default for EngineerModelConfig {
    fn default() -> Self {
        Self {
            coder:     "culi-coder".to_string(),    // → qveris deepseek-r1
            reviewer:  "culi-pro".to_string(),      // → qveris claude-fable-5
            architect: "culi-ultra".to_string(),    // → qveris claude-opus-4.5
            harness:   "sixth".to_string(),         // harness layer, never Qveris
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig {
                primary: "culi-router".into(),          // CulirouterAPI at :4000
                fallbacks: vec!["ollama".into()],        // Local fallback only
                model: "deepseek-v4-flash".into(),       // Free via Blackbox/Sixth
                api_key: None,
                base_url: Some("http://127.0.0.1:4000".into()),
                max_retries: 3,
                timeout_seconds: 120,
            },
            agent: AgentConfig {
                max_iterations: 25,
                max_tokens_per_call: 4096,
                temperature: 0.7,
                context_window: 128000,
                enable_memory: true,
                enable_graph: true,
            },
            storage: StorageConfig {
                db_path: PathBuf::from("data/culi.db"),
                memory_path: PathBuf::from("data/memory"),
                skills_path: PathBuf::from("skills"),
                plugins_path: PathBuf::from("plugins"),
            },
            ui: UiConfig {
                theme: "dark".into(),
                language: "vi".into(),
                show_token_usage: true,
            },
            data_dir: None,
            sub_agent: SubAgentConfig::default(),
            engineer_models: EngineerModelConfig::default(),
        }
    }
}

impl Config {
    /// Load config từ file, nếu không có thì dùng default
    pub fn load(path: Option<&str>) -> Result<Self> {
        // Try loading .env file first (dotenv pattern)
        Self::load_dotenv();

        match path {
            Some(p) => {
                let content = std::fs::read_to_string(p)?;
                let mut config: Config = toml::from_str(&content)?;
                config.override_from_env();
                Ok(config)
            }
            None => {
                let mut config = Config::default();
                config.override_from_env();
                Ok(config)
            }
        }
    }

    /// Load .env file if it exists (dotenv-style)
    fn load_dotenv() {
        let candidates = [".env", ".env.local", "CULI/.env"];
        for path in &candidates {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let k = key.trim();
                        let v = value.trim().trim_matches('"').trim_matches('\'');
                        // Only set if not already set (env vars take priority)
                        if std::env::var(k).is_err() {
                            std::env::set_var(k, v);
                        }
                    }
                }
                tracing::info!("Loaded environment from {}", path);
                break;
            }
        }
    }

    /// Override config fields from environment variables
    fn override_from_env(&mut self) {
        // Provider overrides
        if let Ok(v) = std::env::var("CULI_PRIMARY_PROVIDER") {
            self.provider.primary = v;
        }
        if let Ok(v) = std::env::var("CULI_MODEL") {
            self.provider.model = v;
        }
        // CulirouterAPI URL override
        if let Ok(v) = std::env::var("CULI_ROUTER_URL") {
            self.provider.base_url = Some(v);
        }
        if let Ok(v) = std::env::var("CULI_MAX_RETRIES") {
            if let Ok(n) = v.parse() { self.provider.max_retries = n; }
        }
        if let Ok(v) = std::env::var("CULI_TIMEOUT") {
            if let Ok(n) = v.parse() { self.provider.timeout_seconds = n; }
        }

        // Agent overrides
        if let Ok(v) = std::env::var("CULI_MAX_ITERATIONS") {
            if let Ok(n) = v.parse() { self.agent.max_iterations = n; }
        }
        if let Ok(v) = std::env::var("CULI_TEMPERATURE") {
            if let Ok(f) = v.parse() { self.agent.temperature = f; }
        }
        if let Ok(v) = std::env::var("CULI_ENABLE_MEMORY") {
            self.agent.enable_memory = v.eq_ignore_ascii_case("true");
        }

        // UI overrides
        if let Ok(v) = std::env::var("CULI_THEME") {
            self.ui.theme = v;
        }
        if let Ok(v) = std::env::var("CULI_LANGUAGE") {
            self.ui.language = v;
        }

        // Data directory
        if let Ok(v) = std::env::var("CULI_DATA_DIR") {
            self.data_dir = Some(v);
        } else if self.data_dir.is_none() {
            // Default data directory
            let data = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("culi");
            self.data_dir = Some(data.to_string_lossy().to_string());
        }

        // Engineer Models overrides
        if let Ok(v) = std::env::var("CULI_CODER_MODEL")     { self.engineer_models.coder = v; }
        if let Ok(v) = std::env::var("CULI_REVIEWER_MODEL")  { self.engineer_models.reviewer = v; }
        if let Ok(v) = std::env::var("CULI_ARCHITECT_MODEL") { self.engineer_models.architect = v; }
    }
}
