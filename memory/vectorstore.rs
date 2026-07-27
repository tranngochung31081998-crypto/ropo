use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::MemoryEntry;

/// Vector Store for embeddings and similarity search
/// Inspired by agentmemory's storage backend
pub struct VectorStore {
    entries: Vec<MemoryEntry>,
    #[allow(dead_code)]
    dimensions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entry: MemoryEntry,
    pub score: f32,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            dimensions: 384, // Default embedding dimensions
        }
    }

    pub fn with_dimensions(dimensions: usize) -> Self {
        Self {
            entries: Vec::new(),
            dimensions,
        }
    }

    pub fn store(&mut self, entry: MemoryEntry) -> Result<()> {
        info!("Vector store: storing entry {}", entry.id);
        self.entries.push(entry);
        Ok(())
    }

    /// Cosine similarity search
    pub fn search(&self, _query_embedding: &[f32], top_k: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self.entries
            .iter()
            .filter(|e| e.embedding.is_some())
            .filter_map(|entry| {
                let emb = entry.embedding.as_ref()?;
                let score = cosine_similarity(_query_embedding, emb);
                Some(SearchResult {
                    entry: entry.clone(),
                    score,
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}