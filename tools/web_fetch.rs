use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;

use super::{Tool, ToolParameter, ToolResult};

/// Web Fetch Tool - fetch URLs and return content
pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch content from URLs"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "url".into(),
                description: "URL to fetch".into(),
                param_type: "string".into(),
                required: true,
            },
            ToolParameter {
                name: "method".into(),
                description: "HTTP method (GET, POST)".into(),
                param_type: "string".into(),
                required: false,
            },
        ]
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let url = args["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;
        let _method = args["method"].as_str().unwrap_or("GET");

        info!("Web fetch: {}", url);

        // Placeholder - will be implemented with reqwest
        Ok(ToolResult::success(json!({
            "url": url,
            "content": "Placeholder content - web fetch pending reqwest integration",
            "status": 200,
        })))
    }
}
