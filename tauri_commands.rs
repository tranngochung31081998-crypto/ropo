// Tauri IPC Commands - Frontend ↔ Backend communication
// Replaces HTTP API with direct Rust function calls for better performance

use crate::orchestrator::{AgentResponse, Orchestrator};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared application state for Tauri
pub struct TauriAppState {
    pub orchestrator: Arc<Mutex<Orchestrator>>,
}

impl TauriAppState {
    pub fn new(orchestrator: Orchestrator) -> Self {
        Self {
            orchestrator: Arc::new(Mutex::new(orchestrator)),
        }
    }
}

/// Chat request from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
}

/// Chat response to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: String,
    pub session_id: String,
    pub iterations: u32,
    pub tokens_used: u32,
    pub tool_calls: Vec<String>,
}

/// Memory stats response (matches MemoryStats struct)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatsResponse {
    pub working_count: usize,
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub procedural_count: usize,
    pub total_entries: u64,
    pub dedup_skipped: u64,
}

/// Router stats response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStatsResponse {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub uptime_seconds: u64,
}

/// Send chat message and get agent response
#[tauri::command]
pub async fn send_chat(
    request: ChatRequest,
    state: tauri::State<'_, TauriAppState>,
) -> Result<ChatResponse, String> {
    tracing::info!("Tauri: send_chat - {}", request.message);

    let mut orch = state.orchestrator.lock().await;

    match orch.run(&request.message).await {
        Ok(response) => {
            let (content, iterations, tokens, tools) = match response {
                AgentResponse::Complete(output) => (
                    output.content,
                    output.iterations,
                    output.tokens_used,
                    output.tool_calls,
                ),
                AgentResponse::Partial(output) => (
                    format!("{}\n[Partial: max iterations reached]", output.content),
                    output.iterations,
                    output.tokens_used,
                    output.tool_calls,
                ),
                AgentResponse::Error(e) => return Err(format!("Agent error: {}", e)),
            };

            Ok(ChatResponse {
                message: content,
                session_id: orch.session_id.clone(),
                iterations,
                tokens_used: tokens,
                // ExecutedToolCall has field `name`, not `tool_name`
                tool_calls: tools.into_iter().map(|t| t.name).collect(),
            })
        }
        Err(e) => Err(format!("Orchestrator error: {}", e)),
    }
}

/// Get memory statistics
#[tauri::command]
pub async fn get_memory_stats(
    state: tauri::State<'_, TauriAppState>,
) -> Result<MemoryStatsResponse, String> {
    tracing::info!("Tauri: get_memory_stats");

    let orch = state.orchestrator.lock().await;
    let stats = orch.memory.stats();

    Ok(MemoryStatsResponse {
        working_count:    stats.working_count,
        episodic_count:   stats.episodic_count,
        semantic_count:   stats.semantic_count,
        procedural_count: stats.procedural_count,
        total_entries:    stats.total_entries,
        dedup_skipped:    stats.dedup_skipped,
    })
}

/// Get router statistics
#[tauri::command]
pub async fn get_router_stats(
    state: tauri::State<'_, TauriAppState>,
) -> Result<RouterStatsResponse, String> {
    tracing::info!("Tauri: get_router_stats");

    let orch = state.orchestrator.lock().await;
    let metrics = &orch.metrics;

    Ok(RouterStatsResponse {
        total_requests:     metrics.total_conversations,
        successful_requests: metrics.total_conversations,
        failed_requests:    0,
        uptime_seconds: chrono::Utc::now()
            .signed_duration_since(metrics.start_time)
            .num_seconds()
            .max(0) as u64,
    })
}

/// Get health status
#[tauri::command]
pub async fn get_health(
    state: tauri::State<'_, TauriAppState>,
) -> Result<serde_json::Value, String> {
    tracing::info!("Tauri: get_health");

    let orch = state.orchestrator.lock().await;

    Ok(serde_json::json!({
        "status":     "ok",
        "version":    env!("CARGO_PKG_VERSION"),
        "session_id": orch.session_id,
        "mode":       "desktop",
    }))
}

/// Run security audit on path
#[tauri::command]
pub async fn run_audit(path: String) -> Result<String, String> {
    tracing::info!("Tauri: run_audit - {}", path);

    use crate::agents::security_auditor::SecurityAuditor;
    use std::path::Path;

    let auditor = SecurityAuditor::new();
    match auditor.audit_codebase(Path::new(&path)) {
        Ok(report) => {
            let markdown = report.to_markdown();
            if let Err(e) = std::fs::write("audit_report.md", &markdown) {
                tracing::warn!("Failed to save audit report: {}", e);
            }
            Ok(markdown)
        }
        Err(e) => Err(format!("Audit failed: {}", e)),
    }
}

/// Get context summary
#[tauri::command]
pub async fn get_context_summary(
    state: tauri::State<'_, TauriAppState>,
) -> Result<String, String> {
    tracing::info!("Tauri: get_context_summary");

    let orch = state.orchestrator.lock().await;
    Ok(orch.context.get_context_summary())
}
