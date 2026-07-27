pub mod registry;
pub mod executor;
pub mod filesystem;
pub mod terminal;
pub mod web_search;
pub mod web_fetch;
pub mod code_execution;
pub mod graphify;
pub mod chunk_reader;
pub mod search_replace;

pub use registry::*;
pub use executor::*;
pub use filesystem::*;
pub use terminal::*;
pub use web_search::*;
pub use web_fetch::*;
pub use code_execution::*;
pub use graphify::*;
pub use chunk_reader::*;
pub use search_replace::*;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool trait - mọi tool phải implement
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Vec<ToolParameter>;
    
    async fn execute(&self, args: Value) -> Result<ToolResult>;
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl ToolResult {
    pub fn success(data: Value) -> Self {
        Self {
            success: true,
            data,
            error: None,
            duration_ms: 0,
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: Value::Null,
            error: Some(message.to_string()),
            duration_ms: 0,
        }
    }
}
