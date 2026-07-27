use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::MemoryEntry;

/// Procedural Memory - learned workflows and repeatable patterns
pub struct ProceduralMemory {
    workflows: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralState {
    pub workflows: Vec<MemoryEntry>,
}

impl ProceduralMemory {
    pub fn new() -> Self {
        Self {
            workflows: Vec::new(),
        }
    }

    pub fn store(&mut self, entry: MemoryEntry) -> Result<()> {
        info!("Procedural memory store: workflow_id={}", entry.id);
        self.workflows.push(entry);
        Ok(())
    }

    pub fn get_all(&self) -> Result<Vec<MemoryEntry>> {
        Ok(self.workflows.clone())
    }

    pub fn get_all_entries(&self) -> Vec<MemoryEntry> {
        self.workflows.clone()
    }

    pub fn search_by_pattern(&self, pattern: &str) -> Vec<&MemoryEntry> {
        let pattern_lower = pattern.to_lowercase();
        self.workflows
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    pub fn save_state(&self) -> ProceduralState {
        ProceduralState {
            workflows: self.workflows.clone(),
        }
    }

    pub fn len(&self) -> usize {
        self.workflows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.workflows.is_empty()
    }

    pub fn load_state(&mut self, state: ProceduralState) {
        self.workflows = state.workflows;
        info!("Procedural memory loaded: {} workflows", self.workflows.len());
    }
}
