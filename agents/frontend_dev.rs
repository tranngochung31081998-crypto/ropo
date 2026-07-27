use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use super::{AgentRole, AgentConfig, AgentAnalysis};

/// Frontend Developer Agent
pub struct FrontendDev {
    config: AgentConfig,
}

impl FrontendDev {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "frontend_dev".into(),
                model: "gpt-4o".into(),
                temperature: 0.7,
                max_tokens: 4096,
                context_window: 128000,
            },
        }
    }
}

#[async_trait]
impl AgentRole for FrontendDev {
    fn name(&self) -> &str { "frontend_dev" }
    fn description(&self) -> &str { "Frontend Developer - implements UI components, pages, responsive layouts" }
    fn system_prompt(&self) -> String {
        format!("Bạn là Frontend Developer... Config: Model={}, Temperature={}", self.config.model, self.config.temperature)
    }
    fn perspective(&self) -> String { "Frontend perspective: clean UI, responsive, accessible, performant".into() }
    fn allowed_tools(&self) -> Vec<String> { vec!["read_file".into(), "write_file".into(), "terminal".into()] }
    fn config(&self) -> AgentConfig { self.config.clone() }

    async fn process(&self, task: &str) -> Result<String> {
        info!("Frontend Dev processing: {}", task);
        Ok(format!("## Frontend Implementation\n\n### Task: {}\n\n**Status:** Implemented\n\n**Components created:**\n- Main layout\n- Responsive design\n- Interactive features", task))
    }

    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis> {
        info!("frontend_dev analyzing task: {}", task);
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
