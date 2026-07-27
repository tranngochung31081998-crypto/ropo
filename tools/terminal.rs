use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;
use tracing::info;

use super::{Tool, ToolParameter, ToolResult};

/// Terminal Tool - execute shell commands
pub struct TerminalTool;

impl TerminalTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "Execute shell commands and return output"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "command".into(),
                description: "Shell command to execute".into(),
                param_type: "string".into(),
                required: true,
            },
            ToolParameter {
                name: "working_dir".into(),
                description: "Working directory for command execution".into(),
                param_type: "string".into(),
                required: false,
            },
            ToolParameter {
                name: "timeout".into(),
                description: "Timeout in seconds".into(),
                param_type: "number".into(),
                required: false,
            },
        ]
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let command = args["command"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;
        let working_dir = args["working_dir"].as_str();
        let timeout = args["timeout"].as_u64().unwrap_or(30);

        info!("Executing command: {} (timeout: {}s)", command, timeout);

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            cmd.output(),
        ).await??;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        info!("Command completed with exit code: {}", exit_code);

        Ok(ToolResult::success(json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
            "success": output.status.success(),
        })))
    }
}
