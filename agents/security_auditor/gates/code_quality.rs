// Code quality gates implementation

use super::super::models::*;
use super::GateCheck;

/// Gate 9: Error Swallowing Detection
pub struct ErrorSwallowingGate;

impl ErrorSwallowingGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for ErrorSwallowingGate {
    fn gate_id(&self) -> u8 { 9 }
    fn name(&self) -> &str { "Error Swallowing Detection" }
    fn category(&self) -> GateCategory { GateCategory::CodeQuality }
    fn severity(&self) -> Severity { Severity::Medium }
    fn description(&self) -> &str {
        "Detects silently ignored errors with unwrap(), expect(), or let _"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Skip test code
            if trimmed.starts_with("#[test]") || trimmed.starts_with("#[cfg(test)]") {
                continue;
            }

            // Check for unwrap() outside tests
            if trimmed.contains(".unwrap()") && !file_path.contains("test") {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Error swallowed with unwrap()".to_string(),
                    trimmed.to_string(),
                    Some("Use ? operator or proper error handling".to_string()),
                ));
            }

            // Check for expect() - slightly better but still problematic
            if trimmed.contains(".expect(") && !file_path.contains("test") {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Error swallowed with expect()".to_string(),
                    trimmed.to_string(),
                    Some("Use ? operator with proper error context".to_string()),
                ));
            }

            // Check for ignored Result
            if trimmed.starts_with("let _ =") && trimmed.contains("(") {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Potential Result ignored with let _".to_string(),
                    trimmed.to_string(),
                    Some("Handle the Result or use #[must_use]".to_string()),
                ));
            }
        }

        violations
    }
}

/// Gate 10: Magic Numbers
pub struct MagicNumbersGate;

impl MagicNumbersGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for MagicNumbersGate {
    fn gate_id(&self) -> u8 { 10 }
    fn name(&self) -> &str { "Magic Numbers" }
    fn category(&self) -> GateCategory { GateCategory::CodeQuality }
    fn severity(&self) -> Severity { Severity::Low }
    fn description(&self) -> &str {
        "Detects unexplained numeric literals instead of named constants"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let magic_number_patterns = [
            (r">\s*\d{3,}", "Large number without constant"),
            (r"==\s*\d{3,}", "Magic number in comparison"),
            (r"Duration::from_\w+\(\d+\)", "Magic duration value"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            // Skip const declarations
            if line.contains("const ") || line.contains("static ") {
                continue;
            }

            for (pattern_str, issue) in &magic_number_patterns {
                if let Ok(pattern) = regex::Regex::new(pattern_str) {
                    if pattern.is_match(line) {
                        violations.push(self.create_violation(
                            file_path,
                            line_num + 1,
                            format!("Magic number: {}", issue),
                            line.trim().to_string(),
                            Some("Extract to named constant".to_string()),
                        ));
                    }
                }
            }
        }

        violations
    }
}


/// Gate 11: TODO/FIXME in Production Code
pub struct TodoInProductionGate;

impl TodoInProductionGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for TodoInProductionGate {
    fn gate_id(&self) -> u8 { 11 }
    fn name(&self) -> &str { "TODO/FIXME in Production" }
    fn category(&self) -> GateCategory { GateCategory::CodeQuality }
    fn severity(&self) -> Severity { Severity::Medium }
    fn description(&self) -> &str {
        "Detects unresolved TODO or FIXME comments in critical paths"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();

        // Skip test files
        if file_path.contains("test") || file_path.contains("example") {
            return violations;
        }

        for (line_num, line) in content.lines().enumerate() {
            let upper = line.to_uppercase();
            
            if upper.contains("TODO") || upper.contains("FIXME") || upper.contains("HACK") {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Unresolved TODO/FIXME in production code".to_string(),
                    line.trim().to_string(),
                    Some("Complete implementation or create GitHub issue".to_string()),
                ));
            }
        }

        violations
    }
}

/// Gate 12: Missing Error Context
pub struct MissingErrorContextGate;

impl MissingErrorContextGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for MissingErrorContextGate {
    fn gate_id(&self) -> u8 { 12 }
    fn name(&self) -> &str { "Missing Error Context" }
    fn category(&self) -> GateCategory { GateCategory::CodeQuality }
    fn severity(&self) -> Severity { Severity::Low }
    fn description(&self) -> &str {
        "Detects error propagation without context"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let mut in_pub_fn = false;

        for (line_num, line) in content.lines().enumerate() {
            // Track public functions
            if line.trim().starts_with("pub fn ") || line.trim().starts_with("pub async fn ") {
                in_pub_fn = true;
            }

            // Reset on function end
            if in_pub_fn && line.trim() == "}" && !line.starts_with("    ") {
                in_pub_fn = false;
            }

            // Check for bare ? in public functions
            if in_pub_fn && line.ends_with("?;") && 
               !line.contains(".context(") && 
               !line.contains(".with_context(") {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Error propagated without context in public function".to_string(),
                    line.trim().to_string(),
                    Some("Add .context() or .with_context() for better error messages".to_string()),
                ));
            }
        }

        violations
    }
}
