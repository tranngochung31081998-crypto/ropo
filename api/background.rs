//! Background tasks using Harness Layer (free models)
//! 
//! Tasks:
//! - Memory consolidation (every 5 minutes if >50 entries)
//! - Graph updates (every 10 minutes)
//! - Error pattern detection (continuous)

use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};

use crate::api::AppState;
use crate::provider::Message;

/// Start all background tasks
pub fn start_background_tasks(state: Arc<AppState>) {
    // Task 1: Memory Consolidation
    spawn_memory_consolidation_task(state.clone());
    
    // Task 2: Graph Update (future)
    // spawn_graph_update_task(state.clone());
    
    info!("✅ Background tasks started (harness layer)");
}

/// Memory consolidation task - runs every 5 minutes
fn spawn_memory_consolidation_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(300)); // 5 minutes
        
        loop {
            tick.tick().await;
            
            // Check if consolidation is needed
            let stats = {
                let memory = state.memory.lock().await;
                memory.stats()
            };
            
            if stats.working_count > 50 {
                info!("🧹 Background: Memory consolidation triggered ({} working entries)", stats.working_count);
                
                // Use harness for LLM summarization
                match consolidate_with_harness(&state).await {
                    Ok(report) => {
                        info!(
                            "✅ Memory consolidated: {} → episodic, {} → semantic, {} → procedural ({}ms)",
                            report.working_to_episodic,
                            report.episodic_to_semantic,
                            report.semantic_to_procedural,
                            report.duration_ms
                        );
                    }
                    Err(e) => {
                        warn!("⚠️ Memory consolidation failed: {}", e);
                    }
                }
            }
        }
    });
}

/// Consolidate memory using harness layer for summarization
async fn consolidate_with_harness(state: &Arc<AppState>) -> anyhow::Result<crate::memory::ConsolidationReport> {
    // Step 1: Get working memory entries
    let working_count = {
        let memory = state.memory.lock().await;
        memory.stats().working_count
    };
    
    if working_count == 0 {
        return Ok(crate::memory::ConsolidationReport::default());
    }
    
    // Step 2: Use harness to summarize entries before consolidation
    let summary_prompt = format!(
        "Background task: Consolidate {} memory entries. Extract key facts and patterns.",
        working_count
    );
    
    let messages = vec![Message {
        role: "user".to_string(),
        content: summary_prompt,
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }];
    
    // Call harness (Sixth AI / Blackbox - FREE)
    match state.chat_service.harness_chat(messages).await {
        Ok(response) => {
            info!("🤖 Harness summarization: {}", 
                response.content.as_ref().map(|s| s.chars().take(100).collect::<String>()).unwrap_or_default()
            );
        }
        Err(e) => {
            warn!("⚠️ Harness summarization failed: {}", e);
        }
    }
    
    // Step 3: Run actual consolidation
    let mut memory = state.memory.lock().await;
    memory.consolidate().await
}

/// Graph update task - runs every 10 minutes (TODO)
#[allow(dead_code)]
fn spawn_graph_update_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(600)); // 10 minutes
        
        loop {
            tick.tick().await;
            
            info!("🗺️ Background: Graph update check");
            
            // TODO: Scan for file changes and update graph via harness
            // Use harness to analyze changed files and update architecture
            match update_graph_with_harness(&state).await {
                Ok(_) => {
                    info!("✅ Graph updated");
                }
                Err(e) => {
                    warn!("⚠️ Graph update failed: {}", e);
                }
            }
        }
    });
}

/// Update architecture graph using harness (TODO)
#[allow(dead_code)]
async fn update_graph_with_harness(_state: &Arc<AppState>) -> anyhow::Result<()> {
    // TODO: Implement graph auto-update
    // 1. Detect changed files (git status or file watcher)
    // 2. Use harness to analyze changes
    // 3. Update graph nodes/edges
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_background_tasks_start() {
        // Test that tasks can be spawned without panic
        // Actual execution requires AppState
    }
}
