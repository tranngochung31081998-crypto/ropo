use std::path::PathBuf;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs;
use tracing::info;

use super::{Tool, ToolParameter, ToolResult};

/// File System Tool - read, write, search files
pub struct FileSystemTool;

impl FileSystemTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FileSystemTool {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn description(&self) -> &str {
        "Read, write, search, and list files on the local filesystem"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "operation".into(),
                description: "Operation to perform: read, write, search, list, delete".into(),
                param_type: "string".into(),
                required: true,
            },
            ToolParameter {
                name: "path".into(),
                description: "File or directory path".into(),
                param_type: "string".into(),
                required: true,
            },
            ToolParameter {
                name: "content".into(),
                description: "Content to write (for write operation)".into(),
                param_type: "string".into(),
                required: false,
            },
        ]
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let operation = args["operation"].as_str().unwrap_or("read");
        let path = args["path"].as_str().unwrap_or(".");
        let path = PathBuf::from(path);
        
        match operation {
            "read" => {
                let content = fs::read_to_string(&path).await?;
                info!("Read file: {:?}", path);
                Ok(ToolResult::success(json!({
                    "content": content,
                    "path": path.to_string_lossy(),
                })))
            }
            "write" => {
                let content = args["content"].as_str().unwrap_or("");
                
                // Anti-hallucination layer validation
                let path_str = path.to_string_lossy().replace('\\', "/");
                let is_known_layer = path_str.contains("src/provider/") ||
                                     path_str.contains("src/api/") ||
                                     path_str.contains("src/tools/") ||
                                     path_str.contains("src/config/") ||
                                     path_str.contains("src/memory/") ||
                                     path_str.contains("src/graph/") ||
                                     path_str.contains("src/skills/") ||
                                     path_str.contains("src/orchestrator/") ||
                                     path_str.contains("src/agents/") ||
                                     path_str.contains("frontend/src/api/") ||
                                     path_str.contains("frontend/src/components/") ||
                                     path_str.contains("docs/architecture/") ||
                                     path_str.contains("skills/") ||
                                     path_str.ends_with(".md") ||
                                     path_str.ends_with(".toml") ||
                                     path_str.ends_with(".json") ||
                                     path_str.ends_with(".rs") || // fallback for main.rs, lib.rs
                                     path_str.ends_with(".tsx") ||
                                     path_str.ends_with(".ts") ||
                                     path_str.ends_with(".js") ||
                                     path_str.ends_with(".ps1");

                if !is_known_layer {
                    return Ok(ToolResult::error(&format!(
                        "Validation failed: File '{}' doesn't map to any known architecture layer. \
                         Check docs/architecture/culi.c4 before proceeding.",
                        path_str
                    )));
                }

                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                fs::write(&path, content).await?;
                info!("Wrote file: {:?}", path);
                Ok(ToolResult::success(json!({
                    "success": true,
                    "path": path.to_string_lossy(),
                })))
            }
            "list" => {
                let mut entries = Vec::new();
                let mut read_dir = fs::read_dir(&path).await?;
                while let Some(entry) = read_dir.next_entry().await? {
                    entries.push(entry.file_name().to_string_lossy().to_string());
                }
                Ok(ToolResult::success(json!({
                    "entries": entries,
                    "path": path.to_string_lossy(),
                })))
            }
            "delete" => {
                if path.is_dir() {
                    fs::remove_dir_all(&path).await?;
                } else {
                    fs::remove_file(&path).await?;
                }
                Ok(ToolResult::success(json!({
                    "success": true,
                    "path": path.to_string_lossy(),
                })))
            }
            _ => {
                Ok(ToolResult::error(&format!("Unknown operation: {}", operation)))
            }
        }
    }
}
