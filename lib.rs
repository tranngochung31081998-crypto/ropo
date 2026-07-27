pub mod config;
pub mod provider;
pub mod orchestrator;
pub mod agents;
pub mod tools;
pub mod memory;
pub mod skills;
pub mod graph;
pub mod plugin;
pub mod errors;
pub mod subagent;
pub mod api;

#[cfg(feature = "desktop")]
pub mod tauri_commands;

#[cfg(feature = "desktop")]
pub mod tray;
use anyhow::Result;
use config::Config;
use orchestrator::Orchestrator;
use subagent::SubAgent;

/// Khởi tạo CULI engine với config
pub async fn initialize(config_path: Option<&str>) -> Result<Orchestrator> {
    let config = Config::load(config_path)?;
    
    // Initialize the sub-agent (agent's own assistant)
    let data_dir = config.data_dir.clone().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("culi")
            .to_string_lossy()
            .to_string()
    });
    
    let sub_agent = SubAgent::new(&data_dir)?;
    sub_agent.start().await?;
    
    // Pass sub-agent to orchestrator
    let mut orchestrator = Orchestrator::new(config).await?;
    orchestrator.set_sub_agent(sub_agent).await;
    
    Ok(orchestrator)
}
