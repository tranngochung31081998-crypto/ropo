use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::MemoryEntry;

/// Episodic Memory - session summaries và experiences
pub struct EpisodicMemory {
    entries: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicState {
    pub entries: Vec<MemoryEntry>,
}

impl EpisodicMemory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn store(&mut self, entry: MemoryEntry) -> Result<()> {
        info!("Episodic memory store: {}", entry.id);
        self.entries.push(entry);
        Ok(())
    }

    pub fn get_all(&self) -> Result<Vec<MemoryEntry>> {
        Ok(self.entries.clone())
    }

    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries.iter()
            .filter(|e| e.content.to_lowercase().contains(&query_lower))
            .collect()
    }

    pub fn save_state(&self) -> EpisodicState {
        EpisodicState {
            entries: self.entries.clone(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn load_state(&mut self, state: EpisodicState) {
        self.entries = state.entries;
        info!("Episodic memory loaded: {} entries", self.entries.len());
    }
}
