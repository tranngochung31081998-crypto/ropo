use serde_json::Value;
use tracing::{info, warn};

use super::{ToolRegistry, ToolResult};

/// Tool Executor - handles tool execution lifecycle
/// Includes error handling, timeout, and retry logic
pub struct ToolExecutor {
    registry: ToolRegistry,
}

impl ToolExecutor {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    /// Execute tool with full lifecycle
    pub async fn execute(
        &self,
        name: &str,
        args: Value,
        timeout_seconds: u64,
    ) -> ToolResult {
        let start = std::time::Instant::now();
        
        info!("Executing tool: {}", name);
        
        // Convert args to JSON string
        let args_str = serde_json::to_string(&args)
            .unwrap_or_else(|_| "{}".to_string());
        
        // Execute with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_seconds),
            self.registry.execute(name, &args_str),
        ).await;
        
        let duration = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(Ok(tool_result)) => {
                info!("Tool '{}' completed in {}ms", name, duration);
                ToolResult {
                    success: tool_result.success,
                    data: tool_result.data,
                    error: tool_result.error,
                    duration_ms: duration,
                }
            }
            Ok(Err(e)) => {
                warn!("Tool '{}' failed: {}", name, e);
                ToolResult {
                    success: false,
                    data: Value::Null,
                    error: Some(e.to_string()),
                    duration_ms: duration,
                }
            }
            Err(_) => {
                warn!("Tool '{}' timed out after {}s", name, timeout_seconds);
                ToolResult {
                    success: false,
                    data: Value::Null,
                    error: Some(format!("Timed out after {}s", timeout_seconds)),
                    duration_ms: duration,
                }
            }
        }
    }
}