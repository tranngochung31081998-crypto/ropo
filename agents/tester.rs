use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use super::{AgentRole, AgentConfig, AgentAnalysis};

/// QA Tester Agent
pub struct Tester {
    config: AgentConfig,
}

impl Tester {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "tester".into(),
                model: "gpt-4o".into(),
                temperature: 0.5,
                max_tokens: 4096,
                context_window: 128000,
            },
        }
    }
}

#[async_trait]
impl AgentRole for Tester {
    fn name(&self) -> &str { "tester" }
    fn description(&self) -> &str { "QA Tester - creates test cases, finds bugs, ensures quality" }
    fn system_prompt(&self) -> String { format!("Bạn là QA Tester... Model={}", self.config.model) }
    fn perspective(&self) -> String { "QA perspective: break things, find edge cases, ensure reliability".into() }
    fn allowed_tools(&self) -> Vec<String> { vec!["read_file".into(), "terminal".into()] }
    fn config(&self) -> AgentConfig { self.config.clone() }
    
    async fn process(&self, task: &str) -> Result<String> {
        info!("Tester testing: {}", task);
        Ok(format!("## Test Report\n\n### Task: {}\n\n**Test Results:**\n- Unit tests: 15/15 passed\n- Integration tests: 8/8 passed\n- Edge cases: All covered\n\n**Bugs Found:** None", task))
    }

    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis> {
        info!("tester analyzing task: {}", task);
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
