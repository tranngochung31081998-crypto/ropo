// Security Auditor Agent - with 15-gate quality checking system

use anyhow::Result;
use async_trait::async_trait;
use tracing::info;
use std::path::Path;

use super::{AgentRole, AgentConfig, AgentAnalysis};

pub mod models;
pub mod gates;
pub mod gate_checker;

#[cfg(test)]
mod tests;

pub use models::{GateReport, GateViolation, Severity, GateCategory, GateStats};
pub use gate_checker::GateChecker;

/// Security Auditor Agent - tấn công & phòng thủ
/// Now with 15-gate quality checking system
pub struct SecurityAuditor {
    config: AgentConfig,
    gate_checker: GateChecker,
}

impl SecurityAuditor {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "security_auditor".into(),
                model: "gpt-4o".into(),
                temperature: 0.6,
                max_tokens: 4096,
                context_window: 128000,
            },
            gate_checker: GateChecker::new(),
        }
    }

    /// Run full gate-based audit
    pub fn audit_codebase(&self, path: &Path) -> Result<GateReport> {
        info!("🔒 Running 15-gate security audit on {}", path.display());
        self.gate_checker.check_directory(path)
    }

    /// Quick check single file
    pub fn check_file(&self, path: &Path) -> Result<Vec<GateViolation>> {
        self.gate_checker.check_file(path)
    }

    /// List all available gates
    pub fn list_gates(&self) -> Vec<(u8, String, String)> {
        self.gate_checker.list_gates()
    }
}

#[async_trait]
impl AgentRole for SecurityAuditor {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn description(&self) -> &str {
        "Security Auditor - Finds vulnerabilities, performs attack & defense analysis, 15-gate quality checking"
    }

    fn system_prompt(&self) -> String {
        "You are a security auditor agent. Your role is to find vulnerabilities and perform security analysis.".to_string()
    }

    fn perspective(&self) -> String {
        "Security-focused perspective: analyzing potential vulnerabilities, attack vectors, and defensive measures.".to_string()
    }

    fn allowed_tools(&self) -> Vec<String> {
        vec!["file_read".into(), "grep".into(), "audit".into()]
    }

    async fn process(&self, task: &str) -> Result<String> {
        Ok(format!("Security analysis for: {}", task))
    }

    async fn analyze_task(&self, task: &str) -> Result<AgentAnalysis> {
        Ok(AgentAnalysis {
            agent_name: self.name().to_string(),
            perspective: self.perspective(),
            findings: vec![format!("Analyzed task: {}", task)],
            recommendations: vec!["Run 15-gate security audit".to_string()],
            risks: vec![],
            confidence: 0.8,
        })
    }

    fn config(&self) -> AgentConfig {
        self.config.clone()
    }
}
