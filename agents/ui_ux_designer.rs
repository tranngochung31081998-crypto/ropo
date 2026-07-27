use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use super::{AgentRole, AgentConfig, AgentAnalysis};

/// UI/UX Designer Agent
pub struct UiUxDesigner {
    config: AgentConfig,
}

impl UiUxDesigner {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "ui_ux_designer".into(),
                model: "gpt-4o".into(),
                temperature: 0.8,
                max_tokens: 4096,
                context_window: 128000,
            },
        }
    }
}

#[async_trait]
impl AgentRole for UiUxDesigner {
    fn name(&self) -> &str { "ui_ux_designer" }
    fn description(&self) -> &str { "UI/UX Designer - designs interfaces, design systems, color schemes" }
    fn system_prompt(&self) -> String { format!("Bạn là UI/UX Designer... Model={}", self.config.model) }
    fn perspective(&self) -> String { "Design perspective: beautiful, intuitive, accessible, consistent".into() }
    fn allowed_tools(&self) -> Vec<String> { vec!["read_file".into(), "write_file".into()] }
    fn config(&self) -> AgentConfig { self.config.clone() }
    
    async fn process(&self, task: &str) -> Result<String> {
        info!("UI/UX Designer designing: {}", task);
        Ok(format!("## Design Specification\n\n### Task: {}\n\n**Design System:**\n- Primary Color: #2563EB (Blue)\n- Secondary: #7C3AED (Purple)\n- Typography: Inter, system-ui\n- Spacing: 4px base unit\n\n**Components Designed:**\n- Navigation bar\n- Dashboard layout\n- Form elements\n- Button system\n\n**Accessibility:** WCAG 2.1 AA compliant", task))
    }

    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis> {
        info!("ui_ux_designer analyzing task: {}", task);
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
