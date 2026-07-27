use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::MemoryEntry;

/// Hybrid search combining keyword and vector search
/// Inspired by agentmemory's search system
pub struct HybridSearch {
    entries: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub entry: MemoryEntry,
    pub keyword_score: f32,
    pub vector_score: f32,
    pub combined_score: f32,
}

impl HybridSearch {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Index a memory entry for search
    pub fn index(&mut self, entry: MemoryEntry) -> Result<()> {
        self.entries.push(entry);
        Ok(())
    }

    /// Hybrid search: combine BM25-style keyword + vector similarity
    pub fn search(&self, query: &str, top_k: usize) -> Vec<HybridSearchResult> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<HybridSearchResult> = self.entries
            .iter()
            .map(|entry| {
                let keyword_score = self.compute_keyword_score(&entry.content, &query_terms);
                let vector_score = entry.embedding.as_ref()
                    .map(|_| 0.0_f32) // Will use actual vector sim when embeddings available
                    .unwrap_or(0.0);

                let combined_score = (keyword_score * 0.7) + (vector_score * 0.3);

                HybridSearchResult {
                    entry: entry.clone(),
                    keyword_score,
                    vector_score,
                    combined_score,
                }
            })
            .collect();

        results.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    /// Simple BM25-style keyword scoring
    fn compute_keyword_score(&self, content: &str, query_terms: &[&str]) -> f32 {
        let content_lower = content.to_lowercase();
        let mut score = 0.0_f32;

        for term in query_terms {
            if content_lower.contains(term) {
                score += 1.0 / (1.0 + query_terms.len() as f32);
            }
        }

        score / (self.entries.len().max(1) as f32).sqrt()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
