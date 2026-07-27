//! API models for HTTP endpoints
//! Mirrors frontend types for Router API dashboard

use serde::{Deserialize, Serialize};
use crate::provider::composite_tiers::TierConfig;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub memory_entries: usize,
    pub session_id: Option<String>,
}

/// Router status response - mirrors frontend RouterAPI types
#[derive(Debug, Serialize)]
pub struct RouterStatusResponse {
    pub active_tier: String,
    pub fallback_chain: Vec<String>,
    pub total_requests: u64,
    pub total_cost: f64,
    pub uptime: u64,
    pub last_fallback: Option<String>,
    pub avg_response_time: f64,
    pub cost_saved: f64,
    pub uptime_percentage: f64,
    pub tiers: Vec<TierStatusResponse>,
    pub provider_metrics: Vec<ProviderMetricResponse>,
    pub recent_events: Vec<RouterEventResponse>,
}

#[derive(Debug, Serialize)]
pub struct TierStatusResponse {
    pub name: String,
    pub label: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub weight: u32,
    pub cost_limit: f64,
    pub usage: f64,
    pub avg_latency: f64,
    pub avg_cost_per_request: f64,
    pub total_calls: u64,
    pub success_rate: f64,
    pub last_failover: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderMetricResponse {
    pub provider: String,
    pub label: String,
    pub model: String,
    pub calls: u64,
    pub tokens: u64,
    pub cost: f64,
    pub avg_latency: f64,
    pub success_rate: f64,
    pub errors: u64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct RouterEventResponse {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub message: String,
    pub from_tier: Option<String>,
    pub to_tier: Option<String>,
}

/// Chat request from frontend
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub stream: Option<bool>,
    /// Optional model override (e.g. "deepseek-v4-flash", "claude-fable-5")
    pub model: Option<String>,
}

/// Chat response
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub session_id: String,
    pub tokens_used: u32,
    pub provider: String,
    pub model: String,
}

/// Audit report request
#[derive(Debug, Deserialize)]
pub struct AuditRequest {
    pub path: Option<String>,
    pub format: Option<String>, // "json" or "markdown"
}

/// Audit report response
#[derive(Debug, Serialize)]
pub struct AuditResponse {
    pub report: serde_json::Value,
    pub stats: AuditStats,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
pub struct AuditStats {
    pub total_files: u32,
    pub total_violations: u32,
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub by_category: std::collections::HashMap<String, u32>,
}

/// Memory stats response
#[derive(Debug, Serialize)]
pub struct MemoryStatsResponse {
    pub working_count: usize,
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub procedural_count: usize,
    pub total_entries: u64,
    pub dedup_skipped: u64,
}

/// Memory entry response for GET /api/memory/entries
#[derive(Debug, Serialize)]
pub struct MemoryEntryResponse {
    pub id: String,
    pub title: String,
    pub content: String,
    pub memory_type: String,
    pub importance: f32,
    pub timestamp: String,
}

/// Memory entries listing response
#[derive(Debug, Serialize)]
pub struct MemoryEntriesResponse {
    pub tier: String,
    pub entries: Vec<MemoryEntryResponse>,
    pub total: usize,
    pub returned: usize,
}

/// Settings update request
#[derive(Debug, Deserialize)]
pub struct SettingsUpdate {
    pub agent_mode: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub theme: Option<String>,
    pub agent_models: Option<AgentModelMapping>,
}

/// Settings response (returns current config)
#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub agent_mode: String,
    pub model: String,
    pub provider: String,
    pub theme: String,
    pub max_iterations: u32,
    pub temperature: f32,
    pub enable_memory: bool,
    pub enable_graph: bool,
    pub memory_entries: usize,
    pub uptime_seconds: u64,
    pub agent_models: AgentModelMapping,
}

/// Agent-to-Model mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModelMapping {
    pub orchestrator: String,  // Senior - planning & coordination
    pub coder: String,         // Implementation specialist
    pub reviewer: String,      // Code review
    pub security: String,      // Security analysis
    pub architect: String,     // Design decisions
    pub designer: String,      // UI/UX
}

impl Default for AgentModelMapping {
    fn default() -> Self {
        Self {
            orchestrator: "anthropic/claude-opus-4.8".to_string(),
            coder: "deepseek/deepseek-v4-pro".to_string(),
            reviewer: "anthropic/claude-sonnet-5".to_string(),
            security: "anthropic/claude-sonnet-5".to_string(),
            architect: "anthropic/claude-opus-4.8".to_string(),
            designer: "anthropic/claude-fable-5".to_string(),
        }
    }
}

/// Consolidation request
#[derive(Debug, Deserialize)]
pub struct ConsolidateRequest {
    pub tier: Option<String>,  // Specific tier to consolidate (or all)
}

/// Consolidation response
#[derive(Debug, Serialize)]
pub struct ConsolidateResponse {
    pub status: String,
    pub duration_ms: u64,
    pub working_to_episodic: usize,
    pub episodic_to_semantic: usize,
    pub semantic_to_procedural: usize,
    pub total_dedup_skipped: u64,
    pub total_entries: u64,
}

impl From<&TierConfig> for TierStatusResponse {
    fn from(tier: &TierConfig) -> Self {
        TierStatusResponse {
            name: tier.provider_id.clone(),
            label: tier.provider_id.clone(),
            provider: tier.provider_id.clone(),
            model: tier.model.clone(),
            status: "active".to_string(),
            weight: tier.weight,
            cost_limit: tier.cost_limit.unwrap_or(0.0),
            usage: 0.0,
            avg_latency: 0.0,
            avg_cost_per_request: 0.0,
            total_calls: 0,
            success_rate: 100.0,
            last_failover: None,
        }
    }
}
