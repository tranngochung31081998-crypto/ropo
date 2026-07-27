use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Context Manager - quản lý conversation context
/// Inspired by Hermes Agent's context management pattern
pub struct ContextManager {
    active_context: ConversationContext,
    history: Vec<ConversationContext>,
    max_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub messages_count: u32,
    pub tokens_used: u32,
    pub current_task: Option<String>,
    pub metadata: HashMap<String, String>,
    pub agent_contexts: HashMap<String, AgentContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub agent_type: String,
    pub perspective: String,
    pub findings: Vec<String>,
    pub decisions: Vec<String>,
    pub status: String,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            active_context: ConversationContext {
                id: uuid::Uuid::new_v4().to_string(),
                started_at: Utc::now(),
                messages_count: 0,
                tokens_used: 0,
                current_task: None,
                metadata: HashMap::new(),
                agent_contexts: HashMap::new(),
            },
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Get context summary cho LLM prompt
    pub fn get_context_summary(&self) -> String {
        let mut summary = String::from("=== Context Summary ===\n");
        
        summary.push_str(&format!("Session: {}\n", self.active_context.id));
        summary.push_str(&format!("Started: {}\n", self.active_context.started_at));
        summary.push_str(&format!("Messages: {}\n", self.active_context.messages_count));
        summary.push_str(&format!("Tokens: {}\n", self.active_context.tokens_used));
        
        if let Some(ref task) = self.active_context.current_task {
            summary.push_str(&format!("Current Task: {}\n", task));
        }
        
        if !self.active_context.agent_contexts.is_empty() {
            summary.push_str("\nAgent Contexts:\n");
            for (agent, ctx) in &self.active_context.agent_contexts {
                summary.push_str(&format!(
                    "  {} - {}: {} findings, {} decisions\n",
                    agent, ctx.status, ctx.findings.len(), ctx.decisions.len()
                ));
            }
        }
        
        summary
    }

    /// Update context sau mỗi turn
    pub fn update_after_turn(&mut self, tokens: u32) {
        self.active_context.messages_count += 1;
        self.active_context.tokens_used += tokens;
    }

    /// Set current task
    pub fn set_current_task(&mut self, task: &str) {
        self.active_context.current_task = Some(task.to_string());
        info!("Context: Task set to '{}'", task);
    }

    /// Add agent context
    pub fn add_agent_context(&mut self, agent_type: &str, context: AgentContext) {
        self.active_context.agent_contexts.insert(agent_type.to_string(), context);
    }

    /// Save current context to history
    pub fn save_to_history(&mut self) {
        let context = self.active_context.clone();
        self.history.push(context);
        
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
        
        // Reset for new conversation
        self.active_context = ConversationContext {
            id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            messages_count: 0,
            tokens_used: 0,
            current_task: None,
            metadata: HashMap::new(),
            agent_contexts: HashMap::new(),
        };
    }

    /// Search context history
    pub fn search_history(&self, query: &str) -> Vec<&ConversationContext> {
        let lower = query.to_lowercase();
        self.history.iter()
            .filter(|ctx| {
                ctx.current_task.as_ref().map_or(false, |t| t.to_lowercase().contains(&lower))
                    || ctx.metadata.values().any(|v| v.to_lowercase().contains(&lower))
            })
            .collect()
    }

    pub fn active_context(&self) -> &ConversationContext {
        &self.active_context
    }
}
