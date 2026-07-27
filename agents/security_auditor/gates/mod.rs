// Gate check trait and module exports

use super::models::*;

/// Trait for implementing security gates
pub trait GateCheck: Send + Sync {
    /// Gate metadata
    fn gate_id(&self) -> u8;
    fn name(&self) -> &str;
    fn category(&self) -> GateCategory;
    fn severity(&self) -> Severity;
    fn description(&self) -> &str;

    /// Main check function
    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation>;

    /// Helper to create violation
    fn create_violation(
        &self,
        file_path: &str,
        line: usize,
        message: String,
        snippet: String,
        suggested_fix: Option<String>,
    ) -> GateViolation {
        GateViolation {
            gate_id: self.gate_id(),
            gate_name: self.name().to_string(),
            severity: self.severity(),
            file: file_path.to_string(),
            line,
            column: None,
            message,
            snippet,
            suggested_fix,
            auto_fixable: false,
        }
    }
}

pub mod security;
pub mod performance;
pub mod code_quality;
pub mod architecture;
