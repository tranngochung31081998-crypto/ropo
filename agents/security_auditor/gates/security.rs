// Security gates implementation

use regex::Regex;
use super::super::models::*;
use super::GateCheck;

/// Gate 1: Hardcoded Secrets Detection
pub struct HardcodedSecretsGate {
    patterns: Vec<(Regex, &'static str)>,
}

impl HardcodedSecretsGate {
    pub fn new() -> Self {
        let patterns = vec![
            (Regex::new(r#"sk-[a-zA-Z0-9]{20,}"#).unwrap(), "OpenAI API key"),
            (Regex::new(r#"Bearer [a-zA-Z0-9_-]{20,}"#).unwrap(), "Bearer token"),
            (Regex::new(r#"password\s*=\s*"[^"]{8,}""#).unwrap(), "Hardcoded password"),
            (Regex::new(r#"api_key\s*=\s*"[^"]{20,}""#).unwrap(), "API key"),
            (Regex::new(r#"postgres://[^:]+:[^@]+@"#).unwrap(), "Database URL"),
            (Regex::new(r#"mongodb://[^:]+:[^@]+@"#).unwrap(), "MongoDB URL"),
        ];
        Self { patterns }
    }
}

impl GateCheck for HardcodedSecretsGate {
    fn gate_id(&self) -> u8 { 1 }
    fn name(&self) -> &str { "Hardcoded Secrets Detection" }
    fn category(&self) -> GateCategory { GateCategory::Security }
    fn severity(&self) -> Severity { Severity::Critical }
    fn description(&self) -> &str {
        "Detects hardcoded secrets like API keys, passwords, tokens"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            // Skip comments
            if line.trim().starts_with("//") || line.trim().starts_with("#") {
                continue;
            }

            for (pattern, secret_type) in &self.patterns {
                if pattern.is_match(line) {
                    violations.push(self.create_violation(
                        file_path,
                        line_num + 1,
                        format!("Hardcoded {} detected", secret_type),
                        line.trim().to_string(),
                        Some(format!(
                            "Use env::var(\"{}_KEY\") or config file instead",
                            secret_type.to_uppercase().replace(" ", "_")
                        )),
                    ));
                }
            }
        }

        violations
    }
}


/// Gate 2: SQL Injection Prevention
pub struct SqlInjectionGate {
    pattern: Regex,
}

impl SqlInjectionGate {
    pub fn new() -> Self {
        Self {
            pattern: Regex::new(
                r#"(format!|String::from|push_str).*\s*(SELECT|INSERT|UPDATE|DELETE)"#
            ).unwrap(),
        }
    }
}

impl GateCheck for SqlInjectionGate {
    fn gate_id(&self) -> u8 { 2 }
    fn name(&self) -> &str { "SQL Injection Prevention" }
    fn category(&self) -> GateCategory { GateCategory::Security }
    fn severity(&self) -> Severity { Severity::Critical }
    fn description(&self) -> &str {
        "Detects potential SQL injection through string concatenation"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if self.pattern.is_match(line) {
                violations.push(self.create_violation(
                    file_path,
                    line_num + 1,
                    "Potential SQL injection: string concatenation in query".to_string(),
                    line.trim().to_string(),
                    Some("Use sqlx::query!() with parameter binding ($1, $2, etc.)".to_string()),
                ));
            }
        }

        violations
    }
}

/// Gate 3: Auth Bypass Detection
pub struct AuthBypassGate;

impl AuthBypassGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for AuthBypassGate {
    fn gate_id(&self) -> u8 { 3 }
    fn name(&self) -> &str { "Authentication Bypass Detection" }
    fn category(&self) -> GateCategory { GateCategory::Security }
    fn severity(&self) -> Severity { Severity::Critical }
    fn description(&self) -> &str {
        "Detects commented out auth checks or always-true conditions"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let patterns = [
            (r"//.*authenticate", "Commented auth check"),
            (r"//.*verify.*token", "Commented token verification"),
            (r"if\s+true\s*\{.*auth", "Always-true auth condition"),
            (r"return\s+Ok\(.*\).*//.*TODO.*auth", "TODO in auth logic"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            for (pattern_str, issue) in &patterns {
                if let Ok(pattern) = Regex::new(pattern_str) {
                    if pattern.is_match(line) {
                        violations.push(self.create_violation(
                            file_path,
                            line_num + 1,
                            format!("Auth bypass risk: {}", issue),
                            line.trim().to_string(),
                            Some("Implement proper authentication check".to_string()),
                        ));
                    }
                }
            }
        }

        violations
    }
}


/// Gate 4: Unsafe Deserialization
pub struct UnsafeDeserializationGate;

impl UnsafeDeserializationGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for UnsafeDeserializationGate {
    fn gate_id(&self) -> u8 { 4 }
    fn name(&self) -> &str { "Unsafe Deserialization" }
    fn category(&self) -> GateCategory { GateCategory::Security }
    fn severity(&self) -> Severity { Severity::High }
    fn description(&self) -> &str {
        "Detects deserialization without validation"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let unsafe_patterns = [
            r"serde_json::from_str.*\.unwrap\(\)",
            r"serde_json::from_slice.*\.unwrap\(\)",
            r"bincode::deserialize.*\.unwrap\(\)",
        ];

        for (line_num, line) in content.lines().enumerate() {
            for pattern_str in &unsafe_patterns {
                if let Ok(pattern) = Regex::new(pattern_str) {
                    if pattern.is_match(line) {
                        violations.push(self.create_violation(
                            file_path,
                            line_num + 1,
                            "Unsafe deserialization without validation".to_string(),
                            line.trim().to_string(),
                            Some("Use proper error handling and validate deserialized data".to_string()),
                        ));
                    }
                }
            }
        }

        violations
    }
}

/// Gate 5: Path Traversal Detection
pub struct PathTraversalGate;

impl PathTraversalGate {
    pub fn new() -> Self { Self }
}

impl GateCheck for PathTraversalGate {
    fn gate_id(&self) -> u8 { 5 }
    fn name(&self) -> &str { "Path Traversal Prevention" }
    fn category(&self) -> GateCategory { GateCategory::Security }
    fn severity(&self) -> Severity { Severity::High }
    fn description(&self) -> &str {
        "Detects file operations with user-controlled paths"
    }

    fn check(&self, content: &str, file_path: &str) -> Vec<GateViolation> {
        let mut violations = Vec::new();
        let dangerous_patterns = [
            (r"fs::read.*\.\.", "Path traversal with '..'"),
            (r"fs::write.*\.\.", "Path traversal with '..'"),
            (r"File::open.*\.\.", "File open with '..'"),
            (r"Path::new\(.*format!", "Dynamic path construction"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            for (pattern_str, issue) in &dangerous_patterns {
                if let Ok(pattern) = Regex::new(pattern_str) {
                    if pattern.is_match(line) {
                        violations.push(self.create_violation(
                            file_path,
                            line_num + 1,
                            format!("Path traversal risk: {}", issue),
                            line.trim().to_string(),
                            Some("Validate and sanitize file paths, use canonicalize()".to_string()),
                        ));
                    }
                }
            }
        }

        violations
    }
}
