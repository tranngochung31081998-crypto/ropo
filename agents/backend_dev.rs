use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use super::{AgentRole, AgentConfig, AgentAnalysis};

/// Backend Developer Agent
pub struct BackendDev {
    config: AgentConfig,
}

impl BackendDev {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "backend_dev".into(),
                model: "gpt-4o".into(),
                temperature: 0.7,
                max_tokens: 4096,
                context_window: 128000,
            },
        }
    }
}

#[async_trait]
impl AgentRole for BackendDev {
    fn name(&self) -> &str { "backend_dev" }
    fn description(&self) -> &str { "Backend Developer - APIs, databases, server logic, system design" }
    fn system_prompt(&self) -> String { format!("Bạn là Backend Developer... Config: Model={}", self.config.model) }
    fn perspective(&self) -> String { "Backend perspective: robust APIs, scalable databases, secure auth".into() }
    fn allowed_tools(&self) -> Vec<String> { vec!["read_file".into(), "write_file".into(), "terminal".into(), "web_fetch".into()] }
    fn config(&self) -> AgentConfig { self.config.clone() }
    
    async fn process(&self, task: &str) -> Result<String> {
        info!("Backend Dev processing: {}", task);
        Ok(format!("## Backend Implementation\n\n### Task: {}\n\n**Status:** Implemented\n\n**Components:**\n- API endpoints\n- Database schema\n- Authentication", task))
    }

    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis> {
        info!("backend_dev analyzing task: {}", task);
        Ok(AgentAnalysis {
            agent_name: self.name().to_string(),
            perspective: self.perspective(),
            findings: vec!["Analyzed: ".to_string() + task.chars().take(100).collect::<String>().as_str()],
            recommendations: vec!["Review the full context for details".to_string()],
            risks: vec!["Incomplete analysis - full LLM integration needed".to_string()],
            confidence: 0.5,
        })
    }
}
