// Architecture gates implementation

use super::super::models::*;
use super::GateCheck;

/// Gate 13: Unnecessary Dependencies
pub struct UnnecessaryDepsGate;

impl UnnecessaryDepsGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for UnnecessaryDepsGate {
    fn gate_id(&self) -> u8 { 13 }
    fn name(&self) -> &str { "Unnecessary Dependencies" }
    fn category(&self) -> GateCategory { GateCategory::Architecture }
    fn severity(&self) -> Severity { Severity::Low }
    fn description(&self) -> &str {
        "Detects heavyweight crates for simple tasks"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();

        // Check for regex used for simple checks
        for (line_num, line) in content.lines().enumerate() {
            if line.contains("use regex::") && 
               (line.contains("is_digit") || line.contains("is_ascii")) {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Regex overkill for simple character check".to_string(),
                    line.trim().to_string(),
                    Some("Use std::char methods instead".to_string()),
                ));
            }
        }

        violations
    }
}

/// Gate 14: God Object/Module
pub struct GodObjectGate;

impl GodObjectGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for GodObjectGate {
    fn gate_id(&self) -> u8 { 14 }
    fn name(&self) -> &str { "God Object/Module" }
    fn category(&self) -> GateCategory { GateCategory::Architecture }
    fn severity(&self) -> Severity { Severity::Medium }
    fn description(&self) -> &str {
        "Detects modules/structs with too many responsibilities"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let line_count = content.lines().count();
        let _fn_count = content.matches("fn ").count();

        // Check file size (warn at 500 lines)
        if line_count > 500 {
            violations.push(self.create_violation(
                file_path,
                1,
                format!("Large module: {} lines", line_count),
                format!("File has {} lines (threshold: 500)", line_count),
                Some("Consider splitting into smaller modules".to_string()),
            ));
        }

        // Check method count per struct
        let mut struct_methods: Vec<usize> = Vec::new();
        let mut current_impl_methods = 0;
        let mut in_impl = false;

        for line in content.lines() {
            if line.trim().starts_with("impl ") {
                in_impl = true;
                current_impl_methods = 0;
            }

            if in_impl && line.contains("fn ") {
                current_impl_methods += 1;
            }

            if in_impl && line.trim() == "}" && !line.starts_with("    ") {
                struct_methods.push(current_impl_methods);
                in_impl = false;
            }
        }

        // Warn if any struct has > 15 methods
        for method_count in struct_methods {
            if method_count > 15 {
                violations.push(self.create_violation(
                    file_path,
                    1,
                    format!("God struct: {} methods", method_count),
                    format!("Struct has {} methods (threshold: 15)", method_count),
                    Some("Extract responsibilities into separate structs/traits".to_string()),
                ));
            }
        }

        violations
    }
}

/// Gate 15: Untestable Code (Hard Dependencies)
pub struct UntestableCodeGate;

impl UntestableCodeGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for UntestableCodeGate {
    fn gate_id(&self) -> u8 { 15 }
    fn name(&self) -> &str { "Untestable Code" }
    fn category(&self) -> GateCategory { GateCategory::Architecture }
    fn severity(&self) -> Severity { Severity::Medium }
    fn description(&self) -> &str {
        "Detects tight coupling to concrete types instead of traits"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();

        // Look for struct fields with concrete external types
        let mut in_struct = false;
        let mut struct_line = 0;

        for (line_num, line) in content.lines().enumerate() {
            if line.trim().starts_with("pub struct ") || line.trim().starts_with("struct ") {
                in_struct = true;
                struct_line = line_num + 1;
            }

            if in_struct && line.trim() == "}" {
                in_struct = false;
            }

            // Check for concrete external types in struct fields
            if in_struct {
                let concrete_types = [
                    "OpenAIProvider",
                    "AnthropicProvider",
                    "HttpClient",
                    "reqwest::Client",
                ];

                for concrete_type in &concrete_types {
                    if line.contains(concrete_type) && !line.contains("dyn ") {
                        violations.push(self.create_violation(
                            file_path,
                            line_num + 1,
                            format!("Hard dependency on {} (line {})", concrete_type, struct_line),
                            line.trim().to_string(),
                            Some(format!("Use trait bound: impl LLMProvider or Box<dyn {}Trait>", concrete_type)),
                        ));
                    }
                }
            }
        }

        violations
    }
}
