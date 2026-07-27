// Gate checker engine - scans code for 15 security/quality gates

use anyhow::Result;
use std::path::Path;
use std::time::Instant;
use tracing::{info, debug, warn};

use super::models::*;
use super::gates::*;

pub struct GateChecker {
    gates: Vec<Box<dyn GateCheck>>,
}

impl GateChecker {
    pub fn new() -> Self {
        Self {
            gates: Self::register_all_gates(),
        }
    }

    fn register_all_gates() -> Vec<Box<dyn GateCheck>> {
        vec![
            // Security gates (5)
            Box::new(security::HardcodedSecretsGate::new()),
            Box::new(security::SqlInjectionGate::new()),
            Box::new(security::AuthBypassGate::new()),
            Box::new(security::UnsafeDeserializationGate::new()),
            Box::new(security::PathTraversalGate::new()),
            
            // Performance gates (3)
            Box::new(performance::NPlusOneQueryGate::new()),
            Box::new(performance::UnboundedAllocationGate::new()),
            Box::new(performance::BlockingInAsyncGate::new()),
            
            // Code quality gates (4)
            Box::new(code_quality::ErrorSwallowingGate::new()),
            Box::new(code_quality::MagicNumbersGate::new()),
            Box::new(code_quality::TodoInProductionGate::new()),
            Box::new(code_quality::MissingErrorContextGate::new()),
            
            // Architecture gates (3)
            Box::new(architecture::UnnecessaryDepsGate::new()),
            Box::new(architecture::GodObjectGate::new()),
            Box::new(architecture::UntestableCodeGate::new()),
        ]
    }

    /// Check single file
    pub fn check_file(&self, path: &Path) -> Result<Vec<GateViolation>> {
        let content = std::fs::read_to_string(path)?;
        let file_path = path.to_str().unwrap_or("");
        let mut violations = Vec::new();

        debug!("Checking file: {}", file_path);

        for gate in &self.gates {
            let gate_violations = gate.check(&content, file_path);
            violations.extend(gate_violations);
        }

        Ok(violations)
    }

    /// Check entire directory recursively
    pub fn check_directory(&self, dir: &Path) -> Result<GateReport> {
        let start = Instant::now();
        info!("🔍 Starting gate check on directory: {}", dir.display());

        let mut all_violations = Vec::new();
        let mut scanned_files = Vec::new();

        // Use walkdir for recursive directory traversal
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                path.is_file() && path.extension() == Some("rs".as_ref())
            })
        {
            let path = entry.path();
            scanned_files.push(path.display().to_string());

            match self.check_file(path) {
                Ok(violations) => {
                    if !violations.is_empty() {
                        debug!("Found {} violations in {}", violations.len(), path.display());
                    }
                    all_violations.extend(violations);
                }
                Err(e) => {
                    warn!("Failed to check {}: {}", path.display(), e);
                }
            }
        }

        let scan_duration_ms = start.elapsed().as_millis() as u64;
        let stats = Self::calculate_stats(&all_violations, scanned_files.len());

        info!(
            "✅ Gate check complete: {} violations in {} files ({} ms)",
            all_violations.len(),
            scanned_files.len(),
            scan_duration_ms
        );

        Ok(GateReport {
            violations: all_violations,
            stats,
            scanned_files,
            scan_duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn calculate_stats(violations: &[GateViolation], file_count: usize) -> GateStats {
        let mut stats = GateStats {
            total_files: file_count,
            total_violations: violations.len(),
            ..Default::default()
        };

        for v in violations {
            match v.severity {
                Severity::Critical => stats.critical += 1,
                Severity::High => stats.high += 1,
                Severity::Medium => stats.medium += 1,
                Severity::Low => stats.low += 1,
            }

            let category = format!("{:?}", v.severity);
            *stats.by_category.entry(category).or_insert(0) += 1;
        }

        stats
    }

    /// Get list of all registered gates
    pub fn list_gates(&self) -> Vec<(u8, String, String)> {
        self.gates.iter().map(|gate| {
            (gate.gate_id(), gate.name().to_string(), gate.description().to_string())
        }).collect()
    }
}
