pub mod engine;
pub mod task_decomposer;
pub mod agent_router;
pub mod context;

pub use engine::*;
pub use task_decomposer::*;
pub use agent_router::*;
pub use context::*;

// Re-export commonly used types
pub use task_decomposer::{DecompositionResult, FocusPlan, TaskAnalysis, TaskType, Complexity};

use serde::{Deserialize, Serialize};

/// Kết quả từ agent loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentResponse {
    Complete(AgentOutput),
    Partial(AgentOutput),
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub content: String,
    pub tool_calls: Vec<ExecutedToolCall>,
    pub iterations: u32,
    pub tokens_used: u32,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: serde_json::Value,
    pub duration_ms: u64,
    pub success: bool,
}

/// Task plan từ task decomposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub subtasks: Vec<SubTask>,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub assigned_agent: Option<String>,
    pub status: TaskStatus,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}
