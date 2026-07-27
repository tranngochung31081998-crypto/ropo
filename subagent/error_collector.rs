use anyhow::Result;
use crate::errors::{ErrorEntry, ErrorMemoryManager, ErrorReflection, ErrorType, classify_error};

/// Error Collector - auto-captures errors during agent operation
pub struct ErrorCollector {
    error_memory: ErrorMemoryManager,
}

impl ErrorCollector {
    pub fn new(error_memory: ErrorMemoryManager) -> Self {
        Self { error_memory }
    }

    /// Record a new error (auto-classifies by message content)
    pub async fn record(
        &mut self,
        error_type_str: String,
        title: &str,
        description: &str,
        context: &str,
        solution: Option<&str>,
        tags: Vec<String>,
    ) -> Result<ErrorEntry> {
        // Try to parse the error type; fall back to auto-classification
        let error_type = match error_type_str.as_str() {
            "auto" | "" => classify_error(description),
            _ => {
                let parsed = crate::errors::parse_error_type_str(&error_type_str);
                if parsed == ErrorType::Unknown && !error_type_str.is_empty() {
                    classify_error(description)
                } else {
                    parsed
                }
            }
        };

        self.error_memory.record_error(
            error_type,
            title,
            description,
            context,
            solution,
            None, // code_snippet
            None, // stack_trace
            tags,
        ).await
    }

    /// Find errors relevant to the current query
    pub async fn find_relevant(&self, query: &str, limit: usize) -> Result<Vec<ErrorEntry>> {
        self.error_memory.find_relevant_errors(query, limit).await
    }

    /// Get lessons learned reflection
    pub async fn reflect(&self) -> Result<ErrorReflection> {
        self.error_memory.reflect().await
    }
}
