use std::collections::HashMap;
use anyhow::Result;
use serde_json::Value;
use tracing::info;

use super::{Tool, ToolResult, FileSystemTool, TerminalTool, WebSearchTool, WebFetchTool, GraphifyTool, ChunkReaderTool, SearchReplaceTool};

/// Tool Registry - quản lý và dispatch tools
/// Inspired by Hermes Agent's tool registry pattern
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        // Đăng ký tools mặc định
        registry.register(Box::new(FileSystemTool::new()));
        registry.register(Box::new(TerminalTool::new()));
        registry.register(Box::new(WebSearchTool::new()));
        registry.register(Box::new(WebFetchTool::new()));
        registry.register(Box::new(GraphifyTool::new()));
        registry.register(Box::new(ChunkReaderTool::new()));
        registry.register(Box::new(SearchReplaceTool::new()));
        
        registry
    }

    /// Đăng ký tool mới
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    /// Get tool definitions cho LLM function calling
    pub fn get_definitions(&self) -> Vec<crate::provider::ToolDefinition> {
        self.tools.values().map(|tool| {
            let params = tool.parameters();
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();
            
            for p in &params {
                let param_schema = serde_json::json!({
                    "type": p.param_type,
                    "description": p.description,
                });
                properties.insert(p.name.clone(), param_schema);
                if p.required {
                    required.push(p.name.clone());
                }
            }

            crate::provider::ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required,
                }),
            }
        }).collect()
    }

    /// Execute tool by name
    pub async fn execute(&self, name: &str, args: &str) -> Result<ToolResult> {
        let tool = self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", name))?;
        
        let args_value: Value = serde_json::from_str(args)
            .unwrap_or(Value::Null);
        
        info!("Executing tool: {} with args: {}", name, args);
        let result = tool.execute(args_value).await?;
        info!("Tool '{}' completed (success: {})", name, result.success);
        
        Ok(result)
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn get_tool(&self, name: &str) -> Option<&Box<dyn Tool>> {
        self.tools.get(name)
    }
}
