use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::MemoryEntry;

/// Working Memory - short-term, current session
/// Stores raw observations and current context (từ agentmemory)
pub struct WorkingMemory {
    entries: Vec<MemoryEntry>,
    capacity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryState {
    pub entries: Vec<MemoryEntry>,
    pub capacity: usize,
}

impl WorkingMemory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            capacity: 100,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity,
        }
    }

    /// Store a new working memory entry
    pub fn store(&mut self, entry: MemoryEntry) -> Result<()> {
        if self.entries.len() >= self.capacity {
            self.entries.remove(0); // FIFO eviction
        }
        info!("Working memory store: len={}", entry.content.len());
        self.entries.push(entry);
        Ok(())
    }

    /// Get current working memory context
    pub fn get_context(&self) -> String {
        self.entries.last()
            .map(|e| e.content.clone())
            .unwrap_or_default()
    }

    /// Get recent entries (last N)
    pub fn recent(&self, n: usize) -> Vec<&MemoryEntry> {
        let n = n.min(self.entries.len());
        self.entries.iter().rev().take(n).collect()
    }

    /// Get all entries
    pub fn get_all_entries(&self) -> Vec<MemoryEntry> {
        self.entries.clone()
    }

    /// Clear working memory (called at session end)
    pub fn clear(&mut self) -> Result<Vec<MemoryEntry>> {
        let entries = std::mem::take(&mut self.entries);
        info!("Working memory cleared: {} entries", entries.len());
        Ok(entries)
    }

    /// Get total entries count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if memory is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Save current state
    pub fn save_state(&self) -> WorkingMemoryState {
        WorkingMemoryState {
            entries: self.entries.clone(),
            capacity: self.capacity,
        }
    }

    /// Load state from previous session
    pub fn load_state(&mut self, state: WorkingMemoryState) {
        self.entries = state.entries;
        self.capacity = state.capacity;
        info!("Working memory loaded: {} entries", self.entries.len());
    }
}
