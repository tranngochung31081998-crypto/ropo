pub mod error_collector;
pub mod context_compressor;
pub mod session_manager;

pub use error_collector::*;
pub use context_compressor::*;
pub use session_manager::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::errors::ErrorMemoryManager;

/// SubAgent - Agent's own assistant running in background
/// 
/// Responsibilities (inspired by graphify-8's hooks & claude-howto's subagent pattern):
/// 1. Error Memory Management - auto-capture, classify, store errors
/// 2. Context Compression - detect near-limit context, auto-compress
/// 3. Session Summarization - auto-summarize when session ends
/// 4. Fast Context Retrieval - respond to quick context queries
/// 5. Proactive Error Prevention - inject relevant past errors
pub struct SubAgent {
    name: String,
    error_collector: ErrorCollector,
    context_compressor: ContextCompressor,
    session_manager: SessionManager,
    running: Arc<Mutex<bool>>,
}

impl SubAgent {
    pub fn new(data_dir: &str) -> Result<Self> {
        let error_memory = ErrorMemoryManager::new(data_dir)?;

        Ok(Self {
            name: "assistant".to_string(),
            error_collector: ErrorCollector::new(error_memory),
            context_compressor: ContextCompressor::new(150_000), // Trigger at 150k tokens
            session_manager: SessionManager::new(data_dir)?,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start the sub-agent background loop
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = true;
        tracing::info!("SubAgent '{}' started", self.name);
        Ok(())
    }

    /// Stop the sub-agent
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.lock().await;
        *running = false;
        tracing::info!("SubAgent '{}' stopped", self.name);
        Ok(())
    }

    /// Check if sub-agent is running
    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }

    /// Quick access to error memory
    pub fn error_memory(&self) -> &ErrorCollector {
        &self.error_collector
    }

    /// Quick access to context compressor
    pub fn context_compressor(&self) -> &ContextCompressor {
        &self.context_compressor
    }

    /// Quick access to session manager
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Handle a sub-agent request from the main orchestrator
    pub async fn handle_request(&mut self, request: SubAgentRequest) -> Result<SubAgentResponse> {
        match request {
            SubAgentRequest::GetRelevantErrors { query, limit } => {
                let errors = self.error_collector.find_relevant(&query, limit).await?;
                Ok(SubAgentResponse::ErrorList(errors))
            }
            SubAgentRequest::RecordError { 
                error_type, title, description, context, solution, tags 
            } => {
                self.error_collector.record(
                    error_type, &title, &description, &context, 
                    solution.as_deref(), tags
                ).await?;
                Ok(SubAgentResponse::Acknowledged)
            }
            SubAgentRequest::CompressContext { messages, max_tokens } => {
                let compressed = self.context_compressor.compress(&messages, max_tokens).await?;
                Ok(SubAgentResponse::CompressedContext(compressed))
            }
            SubAgentRequest::CheckContextBudget { current_tokens } => {
                let needs_compression = self.context_compressor.check_budget(current_tokens);
                Ok(SubAgentResponse::BudgetCheck {
                    needs_compression,
                    suggested_max: if needs_compression { 
                        Some(self.context_compressor.target_tokens) 
                    } else { 
                        None 
                    },
                })
            }
            SubAgentRequest::SummarizeSession { session_id } => {
                let summary = self.session_manager.summarize(&session_id).await?;
                Ok(SubAgentResponse::SessionSummary(summary))
            }
            SubAgentRequest::GetFastContext { project_path } => {
                let context = FastContext::new(&project_path).await;
                Ok(SubAgentResponse::FastContext(context))
            }
            SubAgentRequest::Reflect(()) => {
                let reflection = self.error_collector.reflect().await?;
                Ok(SubAgentResponse::Reflection(reflection))
            }
        }
    }
}

/// Requests that the SubAgent can handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubAgentRequest {
    GetRelevantErrors {
        query: String,
        limit: usize,
    },
    RecordError {
        error_type: String,
        title: String,
        description: String,
        context: String,
        solution: Option<String>,
        tags: Vec<String>,
    },
    CompressContext {
        messages: Vec<String>,
        max_tokens: usize,
    },
    CheckContextBudget {
        current_tokens: usize,
    },
    SummarizeSession {
        session_id: String,
    },
    GetFastContext {
        project_path: String,
    },
    Reflect(()),
}

/// Responses from the SubAgent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubAgentResponse {
    ErrorList(Vec<crate::errors::ErrorEntry>),
    Acknowledged,
    CompressedContext(Vec<String>),
    BudgetCheck {
        needs_compression: bool,
        suggested_max: Option<usize>,
    },
    SessionSummary(String),
    FastContext(FastContext),
    Reflection(crate::errors::ErrorReflection),
}

/// Fast project context - quick snapshot of project structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastContext {
    pub project_name: String,
    pub language: String,
    pub file_count: usize,
    pub top_files: Vec<String>,
    pub entry_points: Vec<String>,
    pub dependencies: Vec<String>,
    pub recent_changes: Vec<String>,
    pub build_status: String,
    pub timestamp: String,
}

impl FastContext {
    pub async fn new(project_path: &str) -> Self {
        let path = std::path::Path::new(project_path);
        
        // Count files
        let file_count = Self::count_files(path).await;
        let top_files = Self::find_top_files(path).await;
        let entry_points = Self::find_entry_points(path).await;
        let deps = Self::find_dependencies(path).await;
        
        // Detect language
        let language = Self::detect_language(path);
        let project_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        FastContext {
            project_name,
            language,
            file_count,
            top_files,
            entry_points,
            dependencies: deps,
            recent_changes: Vec::new(),
            build_status: "unknown".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    async fn count_files(path: &std::path::Path) -> usize {
        if !path.is_dir() {
            return 0;
        }
        let mut count = 0;
        if let Ok(entries) = tokio::fs::read_dir(path).await {
            let mut entries = entries;
            loop {
                match entries.next_entry().await {
                    Ok(Some(entry)) => {
                        if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                            count += 1;
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }
        count
    }

    async fn find_top_files(path: &std::path::Path) -> Vec<String> {
        let mut files = Vec::new();
        let interesting = [
            "Cargo.toml", "package.json", "pyproject.toml", "go.mod",
            "README.md", "main.rs", "lib.rs", "index.js", "index.ts",
            "main.py", "app.py", "setup.py", "Makefile", "Dockerfile",
        ];
        for name in &interesting {
            if path.join(name).exists() {
                files.push(name.to_string());
            }
        }
        files
    }

    async fn find_entry_points(path: &std::path::Path) -> Vec<String> {
        let mut entries = Vec::new();
        let entry_names = ["main.rs", "lib.rs", "main.py", "index.js", "index.ts", "app.py", "cli.py"];
        for name in &entry_names {
            if path.join(name).exists() {
                entries.push(name.to_string());
            }
        }
        entries
    }

    async fn find_dependencies(path: &std::path::Path) -> Vec<String> {
        let mut deps = Vec::new();
        
        // Cargo.toml
        let cargo_path = path.join("Cargo.toml");
        if cargo_path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&cargo_path).await {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("name =") {
                        deps.push(trimmed.to_string());
                        break;
                    }
                }
            }
        }

        deps
    }

    fn detect_language(path: &std::path::Path) -> String {
        if path.join("Cargo.toml").exists() { return "Rust".to_string(); }
        if path.join("package.json").exists() { return "JavaScript/TypeScript".to_string(); }
        if path.join("pyproject.toml").exists() || path.join("setup.py").exists() { return "Python".to_string(); }
        if path.join("go.mod").exists() { return "Go".to_string(); }
        if path.join("CMakeLists.txt").exists() { return "C/C++".to_string(); }
        "Unknown".to_string()
    }
}
