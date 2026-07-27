pub mod storage;
pub mod search;
pub mod types;

pub use storage::*;
pub use search::*;
pub use types::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Error Memory Manager - Core của Error Memory System
/// Inspired by graphify-8's `save-result` and `reflect` commands
pub struct ErrorMemoryManager {
    storage: ErrorStorage,
    search: ErrorSearch,
}

impl ErrorMemoryManager {
    pub fn new(data_dir: &str) -> Result<Self> {
        let storage = ErrorStorage::new(data_dir)?;
        let search = ErrorSearch::new(data_dir)?;
        Ok(Self { storage, search })
    }

    /// Ghi nhận một lỗi mới, tự động detect nếu là lỗi lặp lại
    pub async fn record_error(
        &mut self,
        error_type: ErrorType,
        title: &str,
        description: &str,
        context: &str,
        solution: Option<&str>,
        code_snippet: Option<&str>,
        stack_trace: Option<&str>,
        tags: Vec<String>,
    ) -> Result<ErrorEntry> {
        // Check if similar error already exists (dedup)
        let similar = self.search.find_similar(title, 0.85)?;
        if let Some(existing) = similar.first() {
            // Increment frequency
            let mut updated = existing.clone();
            updated.frequency += 1;
            updated.last_seen = chrono::Utc::now().to_rfc3339();
            if let Some(sol) = solution {
                updated.solution = sol.to_string();
            }
            self.storage.update(&updated)?;
            self.search.reindex(&updated)?;
            return Ok(updated);
        }

        let entry = ErrorEntry {
            id: uuid::Uuid::new_v4().to_string(),
            error_type,
            title: title.to_string(),
            description: description.to_string(),
            context: context.to_string(),
            solution: solution.unwrap_or("").to_string(),
            code_snippet: code_snippet.map(|s| s.to_string()),
            stack_trace: stack_trace.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            frequency: 1,
            resolved: solution.is_some(),
            related_errors: Vec::new(),
            tags,
        };

        self.storage.store(&entry)?;
        self.search.index(&entry)?;
        Ok(entry)
    }

    /// Tìm lỗi liên quan đến context hiện tại
    pub async fn find_relevant_errors(&self, query: &str, limit: usize) -> Result<Vec<ErrorEntry>> {
        self.search.hybrid_search(query, limit)
    }

    /// Lấy các lỗi thường gặp nhất (theo frequency)
    pub async fn get_most_frequent(&self, limit: usize) -> Result<Vec<ErrorEntry>> {
        self.storage.get_most_frequent(limit)
    }

    /// Lấy các lỗi gần đây nhất
    pub async fn get_recent(&self, limit: usize) -> Result<Vec<ErrorEntry>> {
        self.storage.get_recent(limit)
    }

    /// Đánh dấu lỗi đã được giải quyết
    pub async fn mark_resolved(&mut self, id: &str, solution: &str) -> Result<()> {
        let mut entry = self.storage.get_by_id(id)?;
        if let Some(ref mut e) = entry {
            e.resolved = true;
            e.solution = solution.to_string();
            self.storage.update(e)?;
            self.search.reindex(e)?;
        }
        Ok(())
    }

    /// Tổng hợp lessons learned (inspired by graphify-8's `reflect`)
    pub async fn reflect(&self) -> Result<ErrorReflection> {
        let entries = self.storage.get_all()?;
        let total = entries.len();
        let resolved = entries.iter().filter(|e| e.resolved).count();
        let unresolved = total - resolved;

        let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut tag_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut top_patterns: Vec<String> = Vec::new();

        for entry in &entries {
            *type_counts.entry(format!("{:?}", entry.error_type)).or_insert(0) += 1;
            for tag in &entry.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
            if entry.frequency > 1 {
                top_patterns.push(format!("{} ({} times)", entry.title, entry.frequency));
            }
        }

        Ok(ErrorReflection {
            total_errors: total,
            resolved_errors: resolved,
            unresolved_errors: unresolved,
            most_common_types: type_counts,
            most_common_tags: tag_counts,
            recurring_patterns: top_patterns,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// Kết quả reflection - tổng hợp kinh nghiệm từ lỗi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReflection {
    pub total_errors: usize,
    pub resolved_errors: usize,
    pub unresolved_errors: usize,
    pub most_common_types: std::collections::HashMap<String, usize>,
    pub most_common_tags: std::collections::HashMap<String, usize>,
    pub recurring_patterns: Vec<String>,
    pub generated_at: String,
}
