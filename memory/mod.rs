pub mod working;
pub mod episodic;
pub mod semantic;
pub mod procedural;
pub mod vectorstore;
pub mod search;
pub mod eviction;
pub mod rrf_search;
pub mod storage;
pub mod harness_consolidator;

pub use working::*;
pub use episodic::*;
pub use semantic::*;
pub use procedural::ProceduralMemory;
pub use vectorstore::*;
pub use search::*;
pub use eviction::*;
pub use rrf_search::*;
pub use storage::*;
pub use harness_consolidator::HarnessConsolidator;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory entry types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryType {
    Working,
    Episodic,
    Semantic,
    Procedural,
}

/// Memory entry with metadata - inspired by agentmemory's CompressedObservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub title: String,
    pub summary: Option<String>,
    pub facts: Vec<String>,
    pub concepts: Vec<String>,
    pub files: Vec<String>,
    pub importance: f32,
    pub timestamp: String,
    pub session_id: String,
    pub source: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: HashMap<String, String>,
}

impl MemoryEntry {
    pub fn new(memory_type: MemoryType, content: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            memory_type,
            content: content.to_string(),
            title: String::new(),
            summary: None,
            facts: Vec::new(),
            concepts: Vec::new(),
            files: Vec::new(),
            importance: 0.5,
            timestamp: chrono::Utc::now().to_rfc3339(),
            session_id: String::new(),
            source: String::new(),
            embedding: None,
            metadata: HashMap::new(),
        }
    }

    /// Create from observation (agentmemory PostToolUse hook pattern)
    pub fn from_observation(
        hook_type: &str,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        session_id: &str,
    ) -> Self {
        let content = format!("[{}] {}: {} => {}", hook_type, tool_name, tool_input, tool_output);
        let title = format!("{}: {}", tool_name, &tool_input.chars().take(60).collect::<String>());
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            memory_type: MemoryType::Working,
            content,
            title,
            summary: None,
            facts: vec![format!("Tool {} was used with input: {}", tool_name, tool_input)],
            concepts: vec![tool_name.to_string()],
            files: Vec::new(),
            importance: 0.3,
            timestamp: chrono::Utc::now().to_rfc3339(),
            session_id: session_id.to_string(),
            source: format!("tool:{}", tool_name),
            embedding: None,
            metadata: HashMap::new(),
        }
    }

    /// Human-readable memory type name
    pub fn memory_type_name(&self) -> &str {
        match self.memory_type {
            MemoryType::Working => "working",
            MemoryType::Episodic => "episodic",
            MemoryType::Semantic => "semantic",
            MemoryType::Procedural => "procedural",
        }
    }

    /// SHA-256 hash for dedup (agentmemory pattern)
    pub fn content_hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(self.content.as_bytes());
        hasher.update(self.source.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Memory pipeline - 4-tier consolidation with dedup and privacy
/// Inspired by agentmemory's design with SHA-256 dedup + privacy filter
pub struct MemoryPipeline {
    pub working: WorkingMemory,
    pub episodic: EpisodicMemory,
    pub semantic: SemanticMemory,
    pub procedural: ProceduralMemory,
    pub vector_store: VectorStore,
    pub search: HybridSearch,
    eviction: EvictionManager,
    /// Dedup map: SHA-256 hash -> last_seen timestamp (agentmemory DedupMap)
    dedup_map: HashMap<String, chrono::DateTime<chrono::Utc>>,
    /// Dedup window: skip re-observation within this window (default 5 min)
    dedup_window_minutes: i64,
    /// Total entries processed
    total_entries: u64,
    /// Total dedup skipped
    dedup_skipped: u64,
    /// SQLite persistent storage (only if data_dir configured)
    storage: Option<MemoryStorage>,
}

impl MemoryPipeline {
    pub fn new() -> Self {
        Self::with_storage(None)
    }

    /// Create with optional SQLite persistence
    pub fn with_storage(storage: Option<MemoryStorage>) -> Self {
        let mut pipeline = Self {
            working: WorkingMemory::with_capacity(100),
            episodic: EpisodicMemory::new(),
            semantic: SemanticMemory::new(),
            procedural: ProceduralMemory::new(),
            vector_store: VectorStore::with_dimensions(384),
            search: HybridSearch::new(),
            eviction: EvictionManager::with_defaults(),
            dedup_map: HashMap::new(),
            dedup_window_minutes: 5,
            total_entries: 0,
            dedup_skipped: 0,
            storage,
        };

        // Rehydrate from persistent storage if available
        if let Some(ref store) = pipeline.storage {
            if let Ok(entries) = store.load_entries_by_type(MemoryType::Working) {
                for entry in entries {
                    let _ = pipeline.working.store(entry);
                }
            }
            if let Ok(entries) = store.load_entries_by_type(MemoryType::Episodic) {
                for entry in entries {
                    let _ = pipeline.episodic.store(entry);
                }
            }
            if let Ok(entries) = store.load_entries_by_type(MemoryType::Semantic) {
                for entry in entries {
                    let _ = pipeline.semantic.store(entry);
                }
            }
            if let Ok(entries) = store.load_entries_by_type(MemoryType::Procedural) {
                for entry in entries {
                    let _ = pipeline.procedural.store(entry);
                }
            }
            tracing::info!("MemoryPipeline rehydrated from SQLite ({}/{}/{}/{})",
                pipeline.working.len(), pipeline.episodic.len(),
                pipeline.semantic.len(), pipeline.procedural.len());
        }

        pipeline
    }

    /// Store observation with dedup + privacy filter
    /// (agentmemory PostToolUse hook pattern)
    pub fn observe(&mut self, entry: MemoryEntry) -> Result<()> {
        self.total_entries += 1;

        // 1. Dedup check (SHA-256, 5min window)
        let hash = entry.content_hash();
        if let Some(last_seen) = self.dedup_map.get(&hash) {
            let elapsed = chrono::Utc::now().signed_duration_since(*last_seen);
            if elapsed.num_minutes() < self.dedup_window_minutes {
                self.dedup_skipped += 1;
                tracing::debug!("Dedup skipped observation: {}", entry.title);
                return Ok(()); // Skip duplicate
            }
        }
        self.dedup_map.insert(hash, chrono::Utc::now());

        // 2. Privacy filter - strip secrets (pattern matching)
        let clean_entry = self.apply_privacy_filter(entry);

        // 3. Store in working memory (clone what we need, pass rest to search)
        let working_entry = MemoryEntry {
            title: clean_entry.title.clone(),
            content: clean_entry.content.clone(),
            facts: clean_entry.facts.clone(),
            concepts: clean_entry.concepts.clone(),
            files: clean_entry.files.clone(),
            importance: clean_entry.importance,
            ..clean_entry.clone()
        };
        
        self.working.store(working_entry)?;
        self.search.index(clean_entry.clone())?;
        
        // 4. Persist to SQLite if storage is available
        if let Some(ref store) = self.storage {
            if let Err(e) = store.store_entry(&clean_entry) {
                tracing::warn!("Failed to persist memory entry to SQLite: {}", e);
            }
        }
        
        Ok(())
    }

    /// Privacy filter - strip API keys, tokens, passwords, etc.
    /// (agentmemory privacy filter pattern)
    fn apply_privacy_filter(&self, mut entry: MemoryEntry) -> MemoryEntry {
        // Use raw string literals with hash delimiters to avoid escaping issues
        let patterns: &[(&str, &str)] = &[
            (r#"(?i)(api[_-]?key\s*[:=]\s*['"]?)[^'"\s]+"#, "$1***REDACTED***"),
            (r#"(?i)(token\s*[:=]\s*['"]?)[^'"\s]+"#, "$1***REDACTED***"),
            (r#"(?i)(secret\s*[:=]\s*['"]?)[^'"\s]+"#, "$1***REDACTED***"),
            (r#"(?i)(password\s*[:=]\s*['"]?)[^'"\s]+"#, "$1***REDACTED***"),
            (r#"(?i)(auth\s*[:=]\s*['"]?)[^'"\s]+"#, "$1***REDACTED***"),
            (r"(?i)(bearer\s+)[a-zA-Z0-9._-]+", "$1***REDACTED***"),
            (r"(?i)(sk-[a-zA-Z0-9]{20,})", "***OPENAI-KEY-REDACTED***"),
            (r"(?i)(sk-ant-[a-zA-Z0-9]{20,})", "***ANTHROPIC-KEY-REDACTED***"),
        ];

        for (pattern, replacement) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                entry.content = re.replace_all(&entry.content, *replacement).to_string();
            }
        }

        entry
    }

    /// 4-tier consolidation from working to procedural memory
    /// (agentmemory consolidation pipeline)
    pub async fn consolidate(&mut self) -> Result<ConsolidationReport> {
        let start = std::time::Instant::now();
        let mut report = ConsolidationReport::default();
        
        // Tier 1: Working -> Episodic 
        // (session-level summary)
        let working_entries = self.working.clear()?;
        report.working_to_episodic = working_entries.len();
        for entry in working_entries {
            self.episodic.store(MemoryEntry {
                memory_type: MemoryType::Episodic,
                ..entry
            })?;
        }
        tracing::info!("Working -> Episodic: {} entries", report.working_to_episodic);

        // Tier 2: Episodic -> Semantic
        // (extract facts, concepts, patterns)
        let episodic_entries = self.episodic.get_all()?;
        report.episodic_to_semantic = episodic_entries.len().min(10); // batch limit
        for entry in episodic_entries.iter().rev().take(10) {
            let mut semantic_entry = MemoryEntry {
                memory_type: MemoryType::Semantic,
                ..entry.clone()
            };
            semantic_entry.importance = self.calculate_importance(entry);
            semantic_entry.facts = self.extract_facts(entry);
            semantic_entry.concepts = self.extract_concepts(entry);
            self.semantic.store(semantic_entry)?;
        }
        tracing::info!("Episodic -> Semantic: {} entries extracted", report.episodic_to_semantic);

        // Tier 3: Semantic -> Procedural
        // (extract repeatable workflows)
        let semantic_entries = self.semantic.get_all()?;
        report.semantic_to_procedural = 0;
        for entry in semantic_entries.iter().rev().take(5) {
            if let Some(workflow) = self.extract_workflow(entry) {
                self.procedural.store(MemoryEntry {
                    memory_type: MemoryType::Procedural,
                    ..workflow
                })?;
                report.semantic_to_procedural += 1;
            }
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        report.total_dedup_skipped = self.dedup_skipped;
        report.total_entries = self.total_entries;

        Ok(report)
    }

    /// Calculate importance score based on frequency and recency
    fn calculate_importance(&self, entry: &MemoryEntry) -> f32 {
        let base = 0.5;
        let frequency_bonus = (entry.facts.len() as f32) * 0.1;
        let recency = {
            let age_hours = match chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
                Ok(ts) => chrono::Utc::now().signed_duration_since(ts.with_timezone(&chrono::Utc)).num_hours() as f32,
                Err(_) => 0.0,
            };
            (1.0 - (age_hours / 24.0).min(1.0)) * 0.3
        };
        (base + frequency_bonus + recency).min(1.0)
    }

    /// Extract facts from memory entry
    fn extract_facts(&self, entry: &MemoryEntry) -> Vec<String> {
        let mut facts = Vec::new();
        // Simple heuristic: split by sentences and keep meaningful ones
        for line in entry.content.lines() {
            let trimmed = line.trim();
            if trimmed.len() > 20 && !trimmed.starts_with('[') {
                facts.push(trimmed.to_string());
            }
        }
        if facts.is_empty() {
            facts.push(entry.content.chars().take(200).collect());
        }
        facts
    }

    /// Extract concepts from memory entry
    fn extract_concepts(&self, entry: &MemoryEntry) -> Vec<String> {
        let mut concepts = Vec::new();
        // Extract code-like identifiers and file paths
        for word in entry.content.split_whitespace() {
            let clean = word.trim_matches(|c: char| c.is_ascii_punctuation());
            if clean.contains('.') || clean.contains("::") || clean.contains('/') {
                if !concepts.contains(&clean.to_string()) {
                    concepts.push(clean.to_string());
                }
            }
        }
        concepts.truncate(10);
        concepts
    }

    /// Extract workflow patterns (multiple similar steps = procedural memory)
    fn extract_workflow(&self, entry: &MemoryEntry) -> Option<MemoryEntry> {
        // Check if entry contains step-like content
        let step_count = entry.content.matches("Step").count() 
            + entry.content.matches("step").count()
            + entry.content.matches("->").count();
        
        if step_count >= 2 {
            let workflow = MemoryEntry {
                content: format!("Workflow: {}\nSteps extracted from: {}", 
                    entry.title, entry.content),
                title: format!("workflow: {}", entry.title),
                concepts: entry.concepts.clone(),
                facts: entry.facts.clone(),
                importance: entry.importance.max(0.7),
                ..entry.clone()
            };
            return Some(workflow);
        }
        None
    }

    /// Hybrid search across all memory tiers using RRF (Reciprocal Rank Fusion)
    /// (agentmemory triple-stream search: BM25 + Vector + Graph)
    pub fn search(&self, query: &str, top_k: usize) -> Vec<search::HybridSearchResult> {
        // Collect all entries from all tiers
        let mut all_entries = Vec::new();
        
        // Add working memory entries
        for entry in self.working.get_all_entries() {
            all_entries.push(entry);
        }
        
        // Add episodic memory entries
        if let Ok(entries) = self.episodic.get_all() {
            all_entries.extend(entries);
        }
        
        // Add semantic memory entries
        if let Ok(entries) = self.semantic.get_all() {
            all_entries.extend(entries);
        }
        
        // Add procedural memory entries
        for entry in self.procedural.get_all_entries() {
            all_entries.push(entry);
        }

        // Run RRF hybrid search on combined entries
        let rrf_results = rrf_search::rrf_hybrid_search(&all_entries, query, None, top_k);
        
        // Convert RRF results back to HybridSearchResult for compatibility
        let results = rrf_results
            .into_iter()
            .map(|r| search::HybridSearchResult {
                entry: r.entry,
                keyword_score: r.bm25_score,
                vector_score: r.vector_score,
                combined_score: r.rrf_score,
            })
            .collect();
        
        results
    }

    /// Get config reference (for eviction settings)
    pub fn eviction_config(&self) -> &eviction::EvictionConfig {
        self.eviction.config()
    }

    /// Run eviction on all memory tiers
    /// Removes expired/low-importance entries based on Ebbinghaus decay curve
    pub fn run_eviction(&mut self) -> EvictionReport {
        let cfg = self.eviction.config();

        let mut report = EvictionReport::default();

        // Evict working memory
        let mut working_entries = self.working.get_all_entries().to_vec();
        let wr = self.eviction.evict(&mut working_entries, cfg.working_ttl_hours, cfg.max_working_entries);
        report.working_evicted = wr.by_ttl + wr.by_importance + wr.by_capacity;

        // Evict episodic memory
        if let Ok(mut entries) = self.episodic.get_all() {
            let er = self.eviction.evict(&mut entries, cfg.episodic_ttl_hours, cfg.max_episodic_entries);
            report.episodic_evicted = er.by_ttl + er.by_importance + er.by_capacity;
            self.episodic = EpisodicMemory::new();
            for entry in entries {
                let _ = self.episodic.store(entry);
            }
        }

        // Evict semantic memory
        if let Ok(mut entries) = self.semantic.get_all() {
            let sr = self.eviction.evict(&mut entries, cfg.semantic_ttl_hours, cfg.max_semantic_entries);
            report.semantic_evicted = sr.by_ttl + sr.by_importance + sr.by_capacity;
            self.semantic = SemanticMemory::new();
            for entry in entries {
                let _ = self.semantic.store(entry);
            }
        }

        // Evict procedural memory
        let mut proc_entries = self.procedural.get_all_entries().to_vec();
        let pr = self.eviction.evict(&mut proc_entries, cfg.procedural_ttl_hours, cfg.max_procedural_entries);
        report.procedural_evicted = pr.by_ttl + pr.by_importance + pr.by_capacity;
        self.procedural = ProceduralMemory::new();
        for entry in proc_entries {
            let _ = self.procedural.store(entry);
        }

        report.total_remaining = self.working.len()
            + self.episodic.len()
            + self.semantic.len()
            + self.procedural.len();

        tracing::info!("Eviction complete: {} removed (W:{} E:{} S:{} P:{})",
            report.working_evicted + report.episodic_evicted + report.semantic_evicted + report.procedural_evicted,
            report.working_evicted, report.episodic_evicted,
            report.semantic_evicted, report.procedural_evicted);

        report
    }

    /// Get pipeline stats
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            working_count: self.working.len(),
            episodic_count: self.episodic.len(),
            semantic_count: self.semantic.len(),
            procedural_count: self.procedural.len(),
            total_entries: self.total_entries,
            dedup_skipped: self.dedup_skipped,
        }
    }
}

/// Consolidation report
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsolidationReport {
    pub working_to_episodic: usize,
    pub episodic_to_semantic: usize,
    pub semantic_to_procedural: usize,
    pub duration_ms: u64,
    pub total_dedup_skipped: u64,
    pub total_entries: u64,
}

/// Memory pipeline statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub working_count: usize,
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub procedural_count: usize,
    pub total_entries: u64,
    pub dedup_skipped: u64,
}
