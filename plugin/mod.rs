pub mod registry;
pub mod lifecycle;
pub mod isolation;
pub mod mcp;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use registry::*;
pub use lifecycle::*;
pub use isolation::*;

/// Plugin descriptor - inspired by Hermes & OpenClaw plugin systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub plugin_type: PluginType,
    pub entry_point: String,
    pub permissions: Vec<String>,
    pub dependencies: Vec<String>,
    pub hooks: Vec<String>,
    pub config_schema: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginType {
    Provider,    // LLM provider
    Tool,        // Tool extension
    Channel,     // Messaging channel
    Memory,      // Memory backend
    Skill,       // Skill pack
    Gateway,     // Gateway extension
    UI,          // UI widget
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginState {
    Discovered,
    Loaded,
    Initialized,
    Running,
    Stopped,
    Error(String),
}

/// Plugin trait - all plugins must implement
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn state(&self) -> PluginState;

    /// Initialize the plugin
    fn init(&mut self, config: HashMap<String, serde_json::Value>) -> Result<()>;

    /// Start the plugin
    fn start(&mut self) -> Result<()>;

    /// Stop the plugin
    fn stop(&mut self) -> Result<()>;

    /// Handle a hook call
    fn handle_hook(&self, hook: &str, payload: &str) -> Result<String>
    where Self: Sized;
}
