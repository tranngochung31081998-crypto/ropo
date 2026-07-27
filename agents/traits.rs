use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Agent Role trait - mọi agent phải implement trait này
/// Inspired by Hermes Agent's plugin system + OpenClaw's plugin SDK pattern
#[async_trait]
pub trait AgentRole: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn system_prompt(&self) -> String;
    fn perspective(&self) -> String;
    fn allowed_tools(&self) -> Vec<String>;
    
    /// Process a task and return result
    async fn process(&self, task: &str) -> Result<String>;

    /// Analyze task from this agent's perspective (returns structured analysis)
    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis>;
    
    /// Get role-specific configuration
    fn config(&self) -> AgentConfig;
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub context_window: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            model: "gpt-4o".into(),
            temperature: 0.7,
            max_tokens: 4096,
            context_window: 128000,
        }
    }
}

/// Agent analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalysis {
    pub agent_name: String,
    pub perspective: String,
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
    pub risks: Vec<String>,
    pub confidence: f32,
}

impl AgentAnalysis {
    pub fn summary(&self) -> String {
        format!(
            "## {} Analysis\n\n**Perspective:** {}\n\n**Findings:**\n{}\n\n**Recommendations:**\n{}\n\n**Risks:**\n{}\n\n**Confidence:** {:.0}%",
            self.agent_name,
            self.perspective,
            self.findings.iter().map(|f| format!("- {}", f)).collect::<Vec<_>>().join("\n"),
            self.recommendations.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n"),
            self.risks.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n"),
            self.confidence * 100.0
        )
    }
}
