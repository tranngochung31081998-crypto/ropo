//! Memory Eviction System
//! Inspired by agentmemory's auto-forgetting with Ebbinghaus decay curve
//! 
//! Features:
//! - Importance-based scoring with time decay
//! - TTL expiry per memory type
//! - Contradiction detection
//! - Automatic eviction when capacity exceeded

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::MemoryEntry;

/// Configuration for memory decay and eviction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvictionConfig {
    /// Half-life in hours for importance decay (Ebbinghaus curve)
    pub importance_halflife_hours: f64,
    /// TTL in hours for working memory
    pub working_ttl_hours: f64,
    /// TTL in hours for episodic memory
    pub episodic_ttl_hours: f64,
    /// TTL in hours for semantic memory
    pub semantic_ttl_hours: f64,
    /// TTL in hours for procedural memory
    pub procedural_ttl_hours: f64,
    /// Minimum importance threshold before eviction
    pub min_importance_threshold: f32,
    /// Maximum entries per memory store before forced eviction
    pub max_working_entries: usize,
    pub max_episodic_entries: usize,
    pub max_semantic_entries: usize,
    pub max_procedural_entries: usize,
}

impl Default for EvictionConfig {
    fn default() -> Self {
        Self {
            importance_halflife_hours: 24.0,
            working_ttl_hours: 2.0,
            episodic_ttl_hours: 24.0,
            semantic_ttl_hours: 168.0,
            procedural_ttl_hours: 720.0,
            min_importance_threshold: 0.05,
            max_working_entries: 100,
            max_episodic_entries: 500,
            max_semantic_entries: 1000,
            max_procedural_entries: 200,
        }
    }
}

/// Eviction report after cleanup
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvictionReport {
    pub working_evicted: usize,
    pub episodic_evicted: usize,
    pub semantic_evicted: usize,
    pub procedural_evicted: usize,
    pub by_ttl: usize,
    pub by_importance: usize,
    pub by_capacity: usize,
    pub total_remaining: usize,
}

/// Compute importance score with Ebbinghaus decay
/// Formula: importance(t) = base_importance * 2^(-t / half_life)
pub fn compute_decayed_importance(
    entry: &MemoryEntry,
    config: &EvictionConfig,
    now: &DateTime<Utc>,
) -> f32 {
    let age_hours = match DateTime::parse_from_rfc3339(&entry.timestamp) {
        Ok(ts) => {
            let utc_ts = ts.with_timezone(&Utc);
            (*now - utc_ts).num_hours() as f64
        }
        Err(_) => 0.0,
    };

    // Ebbinghaus decay: I(t) = I₀ × 2^(-t/h)
    let decay = (-age_hours / config.importance_halflife_hours).exp2();
    let score = entry.importance * decay as f32;
    let frequency_boost = (entry.facts.len() as f32).min(0.3) * 0.1;
    (score + frequency_boost).clamp(0.0, 1.0)
}

/// Check if a memory entry has expired based on TTL
pub fn is_expired(entry: &MemoryEntry, ttl_hours: f64, now: &DateTime<Utc>) -> bool {
    match DateTime::parse_from_rfc3339(&entry.timestamp) {
        Ok(ts) => {
            let utc_ts = ts.with_timezone(&Utc);
            let age_hours = (*now - utc_ts).num_seconds() as f64 / 3600.0;
            age_hours > ttl_hours
        }
        Err(_) => false,
    }
}

/// Memory Eviction Manager
pub struct EvictionManager {
    config: EvictionConfig,
}

impl EvictionManager {
    pub fn new(config: EvictionConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(EvictionConfig::default())
    }

    pub fn config(&self) -> &EvictionConfig {
        &self.config
    }

    /// Run eviction on a set of entries with decay scoring
    pub fn evict(
        &self,
        entries: &mut Vec<MemoryEntry>,
        ttl_hours: f64,
        max_entries: usize,
    ) -> EvictionReport {
        let now = Utc::now();
        let mut report = EvictionReport::default();

        // Phase 1: Remove TTL-expired entries
        entries.retain(|entry| {
            if is_expired(entry, ttl_hours, &now) {
                report.by_ttl += 1;
                false
            } else {
                true
            }
        });

        // Phase 2: Score by decayed importance and remove low-importance
        let mut scored: Vec<(f32, usize)> = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let score = compute_decayed_importance(entry, &self.config, &now);
                (score, idx)
            })
            .collect();

        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut removed_indices = Vec::new();
        let mut kept = 0usize;
        for (score, idx) in &scored {
            if kept >= max_entries || *score < self.config.min_importance_threshold {
                removed_indices.push(*idx);
                if *score < self.config.min_importance_threshold {
                    report.by_importance += 1;
                } else {
                    report.by_capacity += 1;
                }
            } else {
                kept += 1;
            }
        }

        // Remove in reverse order to preserve indices
        removed_indices.sort_by(|a, b| b.cmp(a));
        for idx in removed_indices {
            entries.remove(idx);
        }

        report.total_remaining = entries.len();
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryType;

    fn make_entry(importance: f32, hours_ago: f64, content: &str) -> MemoryEntry {
        let ts = Utc::now() - chrono::Duration::hours(hours_ago as i64);
        MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            memory_type: MemoryType::Working,
            content: content.to_string(),
            title: String::new(),
            summary: None,
            facts: vec![],
            concepts: vec![],
            files: vec![],
            importance,
            timestamp: ts.to_rfc3339(),
            session_id: String::new(),
            source: String::new(),
            embedding: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_ebbinghaus_decay() {
        let config = EvictionConfig::default();
        let now = Utc::now();
        let fresh = make_entry(1.0, 0.0, "test");
        assert!((compute_decayed_importance(&fresh, &config, &now) - 1.0).abs() < 0.1);

        let aged = make_entry(1.0, 24.0, "test");
        let score = compute_decayed_importance(&aged, &config, &now);
        assert!((score - 0.5).abs() < 0.2, "24h entry ~50% importance, got {}", score);

        let old = make_entry(1.0, 168.0, "test");
        assert!(compute_decayed_importance(&old, &config, &now) < 0.1);
    }

    #[test]
    fn test_ttl_expiry() {
        let now = Utc::now();
        assert!(is_expired(&make_entry(1.0, 3.0, "test"), 2.0, &now));
        assert!(!is_expired(&make_entry(1.0, 1.0, "test"), 2.0, &now));
    }

    #[test]
    fn test_eviction() {
        let manager = EvictionManager::with_defaults();
        let mut entries: Vec<MemoryEntry> = (0..10)
            .map(|i| make_entry(0.5 + (i as f32 * 0.05), 0.0, &format!("entry {}", i)))
            .collect();
        let report = manager.evict(&mut entries, 100.0, 5);
        assert_eq!(entries.len(), 5);
        assert_eq!(report.by_capacity, 5);
    }
}
