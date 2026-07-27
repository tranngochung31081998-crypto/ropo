use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::MemoryEntry;

/// Semantic Memory - extracted facts, concepts, and patterns
/// Lưu trữ kiến thức đã được trích xuất từ episodic memory
pub struct SemanticMemory {
    facts: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticState {
    pub facts: Vec<MemoryEntry>,
}

impl SemanticMemory {
    pub fn new() -> Self {
        Self {
            facts: Vec::new(),
        }
    }

    pub fn store(&mut self, entry: MemoryEntry) -> Result<()> {
        info!("Semantic memory store: fact_id={}", entry.id);
        self.facts.push(entry);
        Ok(())
    }

    pub fn get_all(&self) -> Result<Vec<MemoryEntry>> {
        Ok(self.facts.clone())
    }

    pub fn search_by_concept(&self, concept: &str) -> Vec<&MemoryEntry> {
        let concept_lower = concept.to_lowercase();
        self.facts.iter()
            .filter(|e| e.content.to_lowercase().contains(&concept_lower))
            .collect()
    }

    pub fn save_state(&self) -> SemanticState {
        SemanticState {
            facts: self.facts.clone(),
        }
    }

    pub fn len(&self) -> usize {
        self.facts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    pub fn load_state(&mut self, state: SemanticState) {
        self.facts = state.facts;
        info!("Semantic memory loaded: {} facts", self.facts.len());
    }
}
