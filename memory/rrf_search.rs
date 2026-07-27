//! RRF (Reciprocal Rank Fusion) Hybrid Search
//! Inspired by agentmemory's triple-stream retrieval:
//! BM25 keyword + Vector cosine similarity + Knowledge graph traversal
//! 
//! RRF formula: score(d) = Σ(1 / (k + r(d)))
//! where r(d) is the rank of document d, k is a constant (default 60)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::MemoryEntry;

/// RRF rank fusion constant
const RRF_K: f32 = 60.0;

/// Search result from fused streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RRfSearchResult {
    pub entry: MemoryEntry,
    pub bm25_score: f32,
    pub vector_score: f32,
    pub graph_score: f32,
    pub rrf_score: f32,
    pub sources: Vec<String>,
}

/// BM25-style search (keyword matching with TF-IDF weighting)
pub fn bm25_search(entries: &[MemoryEntry], query: &str) -> Vec<(f32, usize)> {
    let query_lower = query.to_lowercase();
    let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
    let n = entries.len() as f32;

    let mut results: Vec<(f32, usize)> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let content_lower = entry.content.to_lowercase();
            let mut score = 0.0f32;

            for term in &query_terms {
                let term_count = content_lower.matches(term).count() as f32;
                if term_count > 0.0 {
                    // TF: term frequency in document
                    let tf = term_count / (content_lower.split_whitespace().count() as f32).max(1.0);
                    // IDF: inverse document frequency
                    let df = entries.iter()
                        .filter(|e| e.content.to_lowercase().contains(term))
                        .count() as f32;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                    score += tf * idf;
                }
            }

            // Title bonus
            let title_lower = entry.title.to_lowercase();
            for term in &query_terms {
                if title_lower.contains(term) {
                    score *= 1.5; // Title match gets 1.5x boost
                    break;
                }
            }

            // Concept bonus
            for concept in &entry.concepts {
                if query_lower.contains(&concept.to_lowercase()) {
                    score *= 1.3;
                    break;
                }
            }

            (score, idx)
        })
        .collect();

    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Simple vector cosine similarity search
pub fn vector_search(entries: &[MemoryEntry], query_embedding: Option<&[f32]>) -> Vec<(f32, usize)> {
    let query_emb = match query_embedding {
        Some(e) => e,
        None => return vec![], // No embeddings configured
    };

    let mut results: Vec<(f32, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| {
            entry.embedding.as_ref().map(|emb| {
                let sim = cosine_similarity(query_emb, emb);
                (sim, idx)
            })
        })
        .collect();

    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Knowledge graph traversal search (by concept matching)
pub fn graph_search(entries: &[MemoryEntry], query: &str) -> Vec<(f32, usize)> {
    let query_lower = query.to_lowercase();
    let query_concepts: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    let mut results: Vec<(f32, usize)> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let mut matched_concepts = 0;
            for concept in &entry.concepts {
                let c_lower = concept.to_lowercase();
                if query_concepts.iter().any(|qc| c_lower.contains(qc) || qc.contains(&c_lower)) {
                    matched_concepts += 1;
                }
            }
            let score = if matched_concepts > 0 && !entry.concepts.is_empty() {
                matched_concepts as f32 / entry.concepts.len() as f32
            } else {
                0.0
            };
            (score, idx)
        })
        .collect();

    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// RRF Fusion: combine multiple ranked result streams
/// 
/// Takes ranked lists from BM25, vector, and graph search,
/// fuses them using Reciprocal Rank Fusion, and returns
/// the top-k results with deduplication.
pub fn rrf_fuse(
    bm25_results: Vec<(f32, usize)>,
    vector_results: Vec<(f32, usize)>,
    graph_results: Vec<(f32, usize)>,
    entries: &[MemoryEntry],
    top_k: usize,
    weights: Option<(f32, f32, f32)>,
) -> Vec<RRfSearchResult> {
    let (w_bm25, w_vec, w_graph) = weights.unwrap_or((1.0, 1.0, 0.5));
    let mut rrf_scores: HashMap<usize, RRfSearchResult> = HashMap::new();

    // Process BM25 stream
    for (rank, (_score, idx)) in bm25_results.iter().enumerate() {
        let rrf_contrib = w_bm25 / (RRF_K + rank as f32 + 1.0);
        let entry = rrf_scores.entry(*idx).or_insert_with(|| RRfSearchResult {
            entry: entries[*idx].clone(),
            bm25_score: 0.0,
            vector_score: 0.0,
            graph_score: 0.0,
            rrf_score: 0.0,
            sources: vec![],
        });
        entry.bm25_score += rrf_contrib;
        entry.rrf_score += rrf_contrib;
        if !entry.sources.contains(&"bm25".to_string()) {
            entry.sources.push("bm25".to_string());
        }
    }

    // Process vector stream
    for (rank, (_score, idx)) in vector_results.iter().enumerate() {
        let rrf_contrib = w_vec / (RRF_K + rank as f32 + 1.0);
        let entry = rrf_scores.entry(*idx).or_insert_with(|| RRfSearchResult {
            entry: entries[*idx].clone(),
            bm25_score: 0.0,
            vector_score: 0.0,
            graph_score: 0.0,
            rrf_score: 0.0,
            sources: vec![],
        });
        entry.vector_score += rrf_contrib;
        entry.rrf_score += rrf_contrib;
        if !entry.sources.contains(&"vector".to_string()) {
            entry.sources.push("vector".to_string());
        }
    }

    // Process graph stream
    for (rank, (_score, idx)) in graph_results.iter().enumerate() {
        let rrf_contrib = w_graph / (RRF_K + rank as f32 + 1.0);
        let entry = rrf_scores.entry(*idx).or_insert_with(|| RRfSearchResult {
            entry: entries[*idx].clone(),
            bm25_score: 0.0,
            vector_score: 0.0,
            graph_score: 0.0,
            rrf_score: 0.0,
            sources: vec![],
        });
        entry.graph_score += rrf_contrib;
        entry.rrf_score += rrf_contrib;
        if !entry.sources.contains(&"graph".to_string()) {
            entry.sources.push("graph".to_string());
        }
    }

    // Sort by RRF score descending
    let mut results: Vec<RRfSearchResult> = rrf_scores.into_values().collect();
    results.sort_by(|a, b| b.rrf_score.partial_cmp(&a.rrf_score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(top_k);
    results
}

/// Perform full RRF hybrid search
/// 
/// Combines BM25 + Vector + Graph search with configurable weights
pub fn rrf_hybrid_search(
    entries: &[MemoryEntry],
    query: &str,
    query_embedding: Option<&[f32]>,
    top_k: usize,
) -> Vec<RRfSearchResult> {
    let bm25 = bm25_search(entries, query);
    let vector = vector_search(entries, query_embedding);
    let graph = graph_search(entries, query);

    rrf_fuse(bm25, vector, graph, entries, top_k, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(content: &str, concepts: Vec<&str>) -> MemoryEntry {
        MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            memory_type: super::super::MemoryType::Working,
            content: content.to_string(),
            title: String::new(),
            summary: None,
            facts: vec![],
            concepts: concepts.iter().map(|s| s.to_string()).collect(),
            files: vec![],
            importance: 0.5,
            timestamp: chrono::Utc::now().to_rfc3339(),
            session_id: String::new(),
            source: String::new(),
            embedding: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_bm25_search() {
        let entries = vec![
            make_entry("The API uses JWT tokens for authentication", vec!["jwt", "auth"]),
            make_entry("Database connection with PostgreSQL", vec!["database", "postgres"]),
            make_entry("Frontend React components with TypeScript", vec!["react", "typescript"]),
        ];

        let results = bm25_search(&entries, "JWT authentication");
        assert!(!results.is_empty());
        assert_eq!(results[0].1, 0, "JWT entry should be top result");
    }

    #[test]
    fn test_graph_search_by_concept() {
        let entries = vec![
            make_entry("JWT middleware setup", vec!["jwt", "auth"]),
            make_entry("React hooks tutorial", vec!["react", "hooks"]),
        ];

        let results = graph_search(&entries, "authentication JWT");
        assert!(!results.is_empty());
        assert_eq!(results[0].1, 0, "JWT entry should match via concepts");
    }

    #[test]
    fn test_rrf_fusion() {
        let entries = vec![
            make_entry("JWT auth implementation", vec!["jwt", "auth"]),
            make_entry("Database setup", vec!["database"]),
            make_entry("React frontend", vec!["react"]),
        ];

        let bm25 = bm25_search(&entries, "JWT");
        let vector = vec![]; // No embeddings
        let graph = graph_search(&entries, "auth");

        let results = rrf_fuse(bm25, vector, graph, &entries, 5, None);
        assert!(!results.is_empty());
        assert_eq!(results[0].entry.content, entries[0].content);
        assert!(results[0].sources.contains(&"bm25".to_string()));
    }

    #[test]
    fn test_full_rrf_hybrid_search() {
        let entries = vec![
            make_entry("JWT token authentication flow", vec!["jwt", "auth", "security"]),
            make_entry("PostgreSQL connection string setup", vec!["database", "postgres"]),
        ];

        let results = rrf_hybrid_search(&entries, "authentication JWT security", None, 5);
        assert_eq!(results.len(), 2);
        assert!(results[0].rrf_score > 0.0);
    }
}
