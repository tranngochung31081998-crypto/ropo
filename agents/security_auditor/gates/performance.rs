// Performance gates implementation

use super::super::models::*;
use super::GateCheck;

/// Gate 6: N+1 Query Detection
pub struct NPlusOneQueryGate;

impl NPlusOneQueryGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for NPlusOneQueryGate {
    fn gate_id(&self) -> u8 { 6 }
    fn name(&self) -> &str { "N+1 Query Detection" }
    fn category(&self) -> GateCategory { GateCategory::Performance }
    fn severity(&self) -> Severity { Severity::High }
    fn description(&self) -> &str {
        "Detects database queries inside loops (N+1 problem)"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let mut loop_depth = 0;
        let mut in_loop = false;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Track loop entry
            if trimmed.starts_with("for ") || trimmed.starts_with("while ") {
                loop_depth += 1;
                in_loop = true;
            }

            // Track loop exit
            if trimmed == "}" && loop_depth > 0 {
                loop_depth -= 1;
                if loop_depth == 0 {
                    in_loop = false;
                }
            }

            // Check for queries inside loops
            if in_loop && (
                line.contains("query!") ||
                line.contains("fetch_one") ||
                line.contains("fetch_optional") ||
                line.contains("execute(")
            ) {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "N+1 query detected: database call inside loop".to_string(),
                    trimmed.to_string(),
                    Some("Use batch query with WHERE id = ANY($1) or JOIN".to_string()),
                ));
            }
        }

        violations
    }
}

/// Gate 7: Unbounded Resource Allocation
pub struct UnboundedAllocationGate;

impl UnboundedAllocationGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for UnboundedAllocationGate {
    fn gate_id(&self) -> u8 { 7 }
    fn name(&self) -> &str { "Unbounded Resource Allocation" }
    fn category(&self) -> GateCategory { GateCategory::Performance }
    fn severity(&self) -> Severity { Severity::High }
    fn description(&self) -> &str {
        "Detects memory allocation without size limits"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            // Check for unbounded Vec allocation
            if (line.contains("Vec::new()") || line.contains("vec![0")) 
                && !line.contains("MAX_") && !line.contains(".min(") {
                // Look for size from request/user input in nearby context
                if line.contains("request") || line.contains("user") || line.contains("input") {
                    violations.push(self.create_violation(
                        file_path,
                        line_num + 1,
                        "Unbounded allocation from user input".to_string(),
                        line.trim().to_string(),
                        Some("Add size limit: let size = user_size.min(MAX_SIZE)".to_string()),
                    ));
                }
            }

            // Check for with_capacity without bounds
            if line.contains("with_capacity") && !line.contains(".min(") {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Potentially unbounded capacity allocation".to_string(),
                    line.trim().to_string(),
                    Some("Add capacity limit constant".to_string()),
                ));
            }
        }

        violations
    }
}


/// Gate 8: Blocking Operations in Async Context
pub struct BlockingInAsyncGate;

impl BlockingInAsyncGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for BlockingInAsyncGate {
    fn gate_id(&self) -> u8 { 8 }
    fn name(&self) -> &str { "Blocking in Async Context" }
    fn category(&self) -> GateCategory { GateCategory::Performance }
    fn severity(&self) -> Severity { Severity::Medium }
    fn description(&self) -> &str {
        "Detects blocking I/O in async functions without spawn_blocking"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let mut in_async_fn = false;
        let mut async_fn_line = 0;

        for (line_num, line) in content.lines().enumerate() {
            // Track async function scope
            if line.contains("async fn ") {
                in_async_fn = true;
                async_fn_line = line_num + 1;
            }

            // Track function end (simplified - looking for closing brace at start of line)
            if in_async_fn && line.trim() == "}" && !line.starts_with("    ") {
                in_async_fn = false;
            }

            // Check for blocking operations in async context
            if in_async_fn {
                let blocking_ops = [
                    ("std::fs::", "Blocking file I/O"),
                    ("std::thread::sleep", "Blocking sleep"),
                    ("std::io::stdin", "Blocking stdin"),
                ];

                for (pattern, op_name) in &blocking_ops {
                    if line.contains(pattern) && !line.contains("spawn_blocking") {
                        violations.push(self.create_violation(
                            file_path,
                            line_num + 1,
                            format!("{} in async function (line {})", op_name, async_fn_line),
                            line.trim().to_string(),
                            Some("Wrap in tokio::task::spawn_blocking()".to_string()),
                        ));
                    }
                }
            }
        }

        violations
    }
}
