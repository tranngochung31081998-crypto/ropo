// Gate checker data models

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

/// Security gate definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityGate {
    pub id: u8,
    pub name: String,
    pub category: GateCategory,
    pub severity: Severity,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateCategory {
    Security,
    Performance,
    CodeQuality,
    Architecture,
}

impl fmt::Display for GateCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateCategory::Security => write!(f, "Security"),
            GateCategory::Performance => write!(f, "Performance"),
            GateCategory::CodeQuality => write!(f, "CodeQuality"),
            GateCategory::Architecture => write!(f, "Architecture"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Critical = 4,  // Must fix before merge
    High = 3,      // Fix in PR review
    Medium = 2,    // Fix in next sprint
    Low = 1,       // Fix when convenient
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Critical => write!(f, "critical"),
            Severity::High => write!(f, "high"),
            Severity::Medium => write!(f, "medium"),
            Severity::Low => write!(f, "low"),
        }
    }
}

/// Gate violation found in code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateViolation {
    pub gate_id: u8,
    pub gate_name: String,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    pub message: String,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<String>,
    pub auto_fixable: bool,
}

/// Complete audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateReport {
    pub violations: Vec<GateViolation>,
    pub stats: GateStats,
    pub scanned_files: Vec<String>,
    pub scan_duration_ms: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateStats {
    pub total_files: usize,
    pub total_violations: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub by_category: HashMap<String, usize>,
}

impl GateReport {
    /// Scanned files count (convenience accessor)
    pub fn scanned_files_count(&self) -> u32 {
        self.stats.total_files as u32
    }

    /// Duration in ms (convenience accessor)
    pub fn duration_ms(&self) -> u64 {
        self.scan_duration_ms
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str("# Security Gate Report\n\n");
        md.push_str(&format!("**Generated:** {}\n", self.timestamp));
        md.push_str(&format!("**Scan Duration:** {} ms\n", self.scan_duration_ms));
        md.push_str(&format!("**Files Scanned:** {}\n\n", self.stats.total_files));

        // Summary stats
        md.push_str("## Summary\n\n");
        md.push_str(&format!("**Total Violations:** {}\n\n", self.stats.total_violations));
        md.push_str(&format!("- 🔴 Critical: {}\n", self.stats.critical));
        md.push_str(&format!("- 🟠 High: {}\n", self.stats.high));
        md.push_str(&format!("- 🟡 Medium: {}\n", self.stats.medium));
        md.push_str(&format!("- 🟢 Low: {}\n\n", self.stats.low));

        // Pass/Fail decision
        if self.stats.critical > 0 {
            md.push_str("**Status:** ❌ FAILED - Critical issues must be fixed\n\n");
        } else if self.stats.high > 0 {
            md.push_str("**Status:** ⚠️ WARNING - High priority issues found\n\n");
        } else {
            md.push_str("**Status:** ✅ PASSED - No critical issues\n\n");
        }

        // Violations by severity
        for severity in [Severity::Critical, Severity::High, Severity::Medium, Severity::Low] {
            let filtered: Vec<_> = self.violations.iter()
                .filter(|v| v.severity == severity)
                .collect();

            if !filtered.is_empty() {
                md.push_str(&format!("\n## {severity:?} Issues ({})\n\n", filtered.len()));

                for violation in filtered {
                    md.push_str(&format!("### Gate #{}: {}\n\n", 
                        violation.gate_id, 
                        violation.gate_name));
                    md.push_str(&format!("**Location:** `{}`:{}\n\n", 
                        violation.file, 
                        violation.line));
                    md.push_str(&format!("**Message:** {}\n\n", violation.message));
                    md.push_str("```rust\n");
                    md.push_str(&violation.snippet);
                    md.push_str("\n```\n\n");

                    if let Some(fix) = &violation.suggested_fix {
                        md.push_str(&format!("**💡 Suggested Fix:** {}\n\n", fix));
                    }

                    md.push_str("---\n\n");
                }
            }
        }

        md
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize report: {}", e))
    }
}
