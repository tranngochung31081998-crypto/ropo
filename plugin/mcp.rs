use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use std::process::{Command, Child};
use std::path::Path;

/// MCP Server configuration
/// Model Context Protocol - enables plugins to communicate via stdio or HTTP
/// Inspired by: https://modelcontextprotocol.io
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub name: String,
    pub transport: MCPTransport,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub enabled: bool,
    pub auto_restart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MCPTransport {
    Stdio,
    Http(String),
    WebSocket(String),
}

/// MCP Tool definition (from MCP server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub server_id: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResource {
    pub server_id: String,
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

/// MCP Server instance (running)
pub struct MCPServerProcess {
    pub config: MCPServerConfig,
    pub child: Option<Child>,
    pub tools: Vec<MCPTool>,
    pub resources: Vec<MCPResource>,
    pub status: MCPServerStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MCPServerStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
    Crashed(String),
}

/// MCP Server Manager - manages discovery, lifecycle, and communication
pub struct MCPServerManager {
    servers: HashMap<String, Arc<Mutex<MCPServerProcess>>>,
    config_dir: String,
}

impl MCPServerManager {
    pub fn new(config_dir: &str) -> Self {
        Self {
            servers: HashMap::new(),
            config_dir: config_dir.to_string(),
        }
    }

    /// Register a new MCP server config
    pub fn register_config(&mut self, config: MCPServerConfig) -> Result<()> {
        let id = config.id.clone();
        info!("Registering MCP server: {} (transport: {:?})", config.name, config.transport);
        let process = MCPServerProcess {
            config,
            child: None,
            tools: Vec::new(),
            resources: Vec::new(),
            status: MCPServerStatus::Stopped,
        };
        self.servers.insert(id, Arc::new(Mutex::new(process)));
        Ok(())
    }

    /// Start an MCP server
    pub async fn start_server(&mut self, id: &str) -> Result<()> {
        let server = self.servers.get(id)
            .ok_or_else(|| anyhow::anyhow!("MCP server not found: {}", id))?;
        let mut process = server.lock().await;

        if process.status == MCPServerStatus::Running {
            warn!("MCP server already running: {}", id);
            return Ok(());
        }

        info!("Starting MCP server: {} ({})", process.config.name, id);
        process.status = MCPServerStatus::Starting;

        let transport_type = process.config.transport.clone();

        match &transport_type {
            MCPTransport::Stdio => {
                let mut cmd = Command::new(&process.config.command);
                cmd.args(&process.config.args);
                cmd.envs(&process.config.env);
                match cmd.spawn() {
                    Ok(child) => {
                        process.child = Some(child);
                        process.status = MCPServerStatus::Running;
                        info!("MCP server started: {}", id);
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to start MCP server {}: {}", id, e);
                        process.status = MCPServerStatus::Error(err_msg.clone());
                        return Err(anyhow::anyhow!(err_msg));
                    }
                }
            }
            MCPTransport::Http(url) => {
                // HTTP transport: ping the URL to verify reachability
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()?;
                match client.get(url).send().await {
                    Ok(_) => {
                        process.status = MCPServerStatus::Running;
                        info!("MCP HTTP server verified: {}", url);
                    }
                    Err(e) => {
                        let err_msg = format!("MCP HTTP server unreachable {}: {}", url, e);
                        process.status = MCPServerStatus::Error(err_msg.clone());
                        return Err(anyhow::anyhow!(err_msg));
                    }
                }
            }
            MCPTransport::WebSocket(url) => {
                // WebSocket: just record as running, actual connect on use
                process.status = MCPServerStatus::Running;
                info!("MCP WebSocket server registered: {}", url);
            }
        }

        Ok(())
    }

    /// Stop an MCP server
    pub async fn stop_server(&mut self, id: &str) -> Result<()> {
        let server = self.servers.get(id)
            .ok_or_else(|| anyhow::anyhow!("MCP server not found: {}", id))?;
        let mut process = server.lock().await;

        if let Some(mut child) = process.child.take() {
            info!("Stopping MCP server: {}", id);
            // Try graceful shutdown first
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                let _ = child.kill();
            }
            #[cfg(windows)]
            {
                let _ = child.kill();
            }
            let _ = child.wait();
            process.status = MCPServerStatus::Stopped;
            info!("MCP server stopped: {}", id);
        } else {
            process.status = MCPServerStatus::Stopped;
        }

        Ok(())
    }

    /// Discover MCP servers from config directory
    pub fn discover_servers(&mut self) -> Result<Vec<MCPServerConfig>> {
        let config_path = Path::new(&self.config_dir);
        let mut discovered = Vec::new();

        if !config_path.exists() {
            info!("MCP config directory not found: {}", self.config_dir);
            return Ok(discovered);
        }

        // Scan for JSON config files in the MCP directory
        if let Ok(entries) = std::fs::read_dir(config_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            match serde_json::from_str::<MCPServerConfig>(&content) {
                                Ok(config) => {
                                    info!("Discovered MCP server: {} from {:?}", config.name, path);
                                    if config.enabled {
                                        self.register_config(config.clone())?;
                                        discovered.push(config);
                                    }
                                }
                                Err(e) => {
                                    warn!("Invalid MCP config in {:?}: {}", path, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Cannot read MCP config {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Get all registered MCP tools
    pub fn all_tools(&self) -> Vec<MCPTool> {
        Vec::new()
    }

    /// Get a specific server's info
    pub fn get_server(&self, id: &str) -> Option<Arc<Mutex<MCPServerProcess>>> {
        self.servers.get(id).cloned()
    }

    /// Get all server IDs
    pub fn server_ids(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }

    /// Number of servers
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }
}

/// MCP Tool Call - execute a tool from an MCP server
pub async fn call_mcp_tool(
    server_process: &Arc<Mutex<MCPServerProcess>>,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<serde_json::Value> {
    let mut process = server_process.lock().await;
    let transport = process.config.transport.clone();
    
    match transport {
        MCPTransport::Stdio => {
            // For stdio transport, write to stdin and read from stdout
            if let Some(ref mut child) = process.child {
                let input = serde_json::json!({
                    "type": "tool_call",
                    "tool": tool_name,
                    "arguments": arguments,
                });

                // Write to child's stdin
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    writeln!(stdin, "{}", serde_json::to_string(&input)?)?;
                }

                // Read from child's stdout (simplified - in production use buffered reading)
                use std::io::Read;
                if let Some(ref mut stdout) = child.stdout {
                    let mut output = String::new();
                    stdout.read_to_string(&mut output)?;
                    let result: serde_json::Value = serde_json::from_str(&output)?;
                    return Ok(result);
                }
            }
            Err(anyhow::anyhow!("MCP server not running"))
        }
        MCPTransport::Http(url) => {
            let client = reqwest::Client::new();
            let full_url = format!("{}/tools/{}", url.trim_end_matches('/'), tool_name);
            let resp = client.post(&full_url)
                .json(arguments)
                .send()
                .await?;
            let result = resp.json::<serde_json::Value>().await?;
            Ok(result)
        }
        MCPTransport::WebSocket(_) => {
            Err(anyhow::anyhow!("WebSocket transport not yet implemented"))
        }
    }
}
