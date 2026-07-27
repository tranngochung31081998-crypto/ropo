//! SQLite persistence layer for Memory Pipeline
//! Stores MemoryEntry across all 4 tiers with automatic dedup and rehydration
//! Inspired by agentmemory's storage backend

use anyhow::{Result, Context};
use rusqlite::{params, Connection};
use serde_json;
use std::path::Path;
use std::sync::Mutex;

use super::{MemoryEntry, MemoryType, MemoryStats};

/// Persistent storage for all 4 memory tiers using SQLite
pub struct MemoryStorage {
    conn: Mutex<Connection>,
}

impl MemoryStorage {
    /// Open (or create) SQLite database at the given path
    pub fn new(db_path: &str) -> Result<Self> {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)
            .context(format!("Failed to open SQLite database at {}", db_path))?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        // Create tables for all 4 memory tiers
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memory_entries (
                id TEXT PRIMARY KEY,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                summary TEXT,
                facts TEXT NOT NULL DEFAULT '[]',
                concepts TEXT NOT NULL DEFAULT '[]',
                files TEXT NOT NULL DEFAULT '[]',
                importance REAL NOT NULL DEFAULT 0.5,
                timestamp TEXT NOT NULL,
                session_id TEXT NOT NULL DEFAULT '',
                source TEXT NOT NULL DEFAULT '',
                embedding BLOB,
                metadata TEXT NOT NULL DEFAULT '{}',
                content_hash TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_entries(memory_type);
            CREATE INDEX IF NOT EXISTS idx_timestamp ON memory_entries(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_importance ON memory_entries(importance DESC);
            CREATE INDEX IF NOT EXISTS idx_content_hash ON memory_entries(content_hash);
            CREATE INDEX IF NOT EXISTS idx_session ON memory_entries(session_id);

            CREATE TABLE IF NOT EXISTS memory_dedup (
                content_hash TEXT PRIMARY KEY,
                last_seen TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS memory_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;

        tracing::info!("MemoryStorage initialized at {}", db_path);
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Store a memory entry (insert or update if hash exists)
    pub fn store_entry(&self, entry: &MemoryEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let hash = entry.content_hash();
        let facts_json = serde_json::to_string(&entry.facts)?;
        let concepts_json = serde_json::to_string(&entry.concepts)?;
        let files_json = serde_json::to_string(&entry.files)?;
        let metadata_json = serde_json::to_string(&entry.metadata)?;
        let memory_type_str = entry.memory_type_name();
        let embedding_blob = entry.embedding.as_ref()
            .map(|v| {
                let bytes: Vec<u8> = v.iter()
                    .flat_map(|f| f.to_le_bytes())
                    .collect();
                bytes
            });

        conn.execute(
            "INSERT OR REPLACE INTO memory_entries
             (id, memory_type, content, title, summary, facts, concepts, files,
              importance, timestamp, session_id, source, embedding, metadata, content_hash, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, datetime('now'))",
            params![
                entry.id,
                memory_type_str,
                entry.content,
                entry.title,
                entry.summary,
                facts_json,
                concepts_json,
                files_json,
                entry.importance,
                entry.timestamp,
                entry.session_id,
                entry.source,
                embedding_blob,
                metadata_json,
                hash,
            ],
        )?;

        // Update dedup map
        conn.execute(
            "INSERT OR REPLACE INTO memory_dedup (content_hash, last_seen) VALUES (?1, ?2)",
            params![hash, chrono::Utc::now().to_rfc3339()],
        )?;

        tracing::debug!("Stored memory entry: {} ({})", entry.id, memory_type_str);
        Ok(())
    }

    /// Store multiple entries in a transaction
    pub fn store_entries(&self, entries: &[MemoryEntry]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("BEGIN TRANSACTION")?;

        for entry in entries {
            let hash = entry.content_hash();
            let facts_json = serde_json::to_string(&entry.facts)?;
            let concepts_json = serde_json::to_string(&entry.concepts)?;
            let files_json = serde_json::to_string(&entry.files)?;
            let metadata_json = serde_json::to_string(&entry.metadata)?;
            let memory_type_str = entry.memory_type_name();

            conn.execute(
                "INSERT OR REPLACE INTO memory_entries
                 (id, memory_type, content, title, summary, facts, concepts, files,
                  importance, timestamp, session_id, source, embedding, metadata, content_hash, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, datetime('now'))",
                params![
                    entry.id, memory_type_str, entry.content, entry.title, entry.summary,
                    facts_json, concepts_json, files_json, entry.importance, entry.timestamp,
                    entry.session_id, entry.source, None::<Vec<u8>>, metadata_json, hash,
                ],
            )?;

            conn.execute(
                "INSERT OR REPLACE INTO memory_dedup (content_hash, last_seen) VALUES (?1, ?2)",
                params![hash, chrono::Utc::now().to_rfc3339()],
            )?;
        }

        conn.execute_batch("COMMIT")?;
        tracing::info!("Stored {} memory entries in transaction", entries.len());
        Ok(())
    }

    /// Load all entries of a specific memory type
    pub fn load_entries_by_type(&self, memory_type: MemoryType) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().unwrap();
        let type_str = match memory_type {
            MemoryType::Working => "working",
            MemoryType::Episodic => "episodic",
            MemoryType::Semantic => "semantic",
            MemoryType::Procedural => "procedural",
        };

        let mut stmt = conn.prepare(
            "SELECT id, memory_type, content, title, summary, facts, concepts, files,
                    importance, timestamp, session_id, source, metadata
             FROM memory_entries WHERE memory_type = ?1 ORDER BY timestamp DESC"
        )?;

        let entries = stmt.query_map(params![type_str], |row| {
            let memory_type_str: String = row.get(1)?;
            let facts_str: String = row.get(5)?;
            let concepts_str: String = row.get(6)?;
            let files_str: String = row.get(7)?;
            let metadata_str: String = row.get(12)?;

            let memory_type = match memory_type_str.as_str() {
                "working" => MemoryType::Working,
                "episodic" => MemoryType::Episodic,
                "semantic" => MemoryType::Semantic,
                "procedural" => MemoryType::Procedural,
                _ => MemoryType::Working,
            };

            Ok(MemoryEntry {
                id: row.get(0)?,
                memory_type,
                content: row.get(2)?,
                title: row.get(3)?,
                summary: row.get(4)?,
                facts: serde_json::from_str(&facts_str).unwrap_or_default(),
                concepts: serde_json::from_str(&concepts_str).unwrap_or_default(),
                files: serde_json::from_str(&files_str).unwrap_or_default(),
                importance: row.get(8)?,
                timestamp: row.get(9)?,
                session_id: row.get(10)?,
                source: row.get(11)?,
                embedding: None,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            })
        })?;

        let result: Vec<MemoryEntry> = entries.filter_map(|e| e.ok()).collect();
        tracing::debug!("Loaded {} entries of type {}", result.len(), type_str);
        Ok(result)
    }

    /// Load all entries across all types
    pub fn load_all_entries(&self) -> Result<Vec<MemoryEntry>> {
        let mut all = Vec::new();
        for mt in &[MemoryType::Working, MemoryType::Episodic, MemoryType::Semantic, MemoryType::Procedural] {
            if let Ok(entries) = self.load_entries_by_type(mt.clone()) {
                all.extend(entries);
            }
        }
        Ok(all)
    }

    /// Get total count per type
    pub fn get_stats(&self) -> Result<MemoryStats> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT memory_type, COUNT(*) FROM memory_entries GROUP BY memory_type"
        )?;

        let mut working_count = 0usize;
        let mut episodic_count = 0usize;
        let mut semantic_count = 0usize;
        let mut procedural_count = 0usize;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            if let Ok((type_str, count)) = row {
                match type_str.as_str() {
                    "working" => working_count = count as usize,
                    "episodic" => episodic_count = count as usize,
                    "semantic" => semantic_count = count as usize,
                    "procedural" => procedural_count = count as usize,
                    _ => {}
                }
            }
        }

        let total = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;

        let dedup = conn.query_row(
            "SELECT COUNT(*) FROM memory_dedup",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;

        Ok(MemoryStats {
            working_count,
            episodic_count,
            semantic_count,
            procedural_count,
            total_entries: total,
            dedup_skipped: dedup.saturating_sub(total),
        })
    }

    /// Delete expired entries (called by EvictionManager)
    pub fn delete_entries_by_ids(&self, ids: &[String]) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut deleted = 0usize;
        for id in ids {
            let affected = conn.execute("DELETE FROM memory_entries WHERE id = ?1", params![id])?;
            deleted += affected;
        }
        Ok(deleted)
    }

    /// Delete all entries of a given type
    pub fn clear_type(&self, memory_type: MemoryType) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let type_str = memory_type.memory_type_name();
        let deleted = conn.execute(
            "DELETE FROM memory_entries WHERE memory_type = ?1",
            params![type_str],
        )?;
        Ok(deleted)
    }

    /// Check if a content hash exists in the dedup map
    pub fn check_dedup(&self, content_hash: &str, _window_minutes: i64) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT last_seen FROM memory_dedup WHERE content_hash = ?1",
            params![content_hash],
            |row| row.get::<_, String>(0),
        ).ok()
    }

    /// Update dedup map entry
    pub fn update_dedup(&self, content_hash: &str, timestamp: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO memory_dedup (content_hash, last_seen) VALUES (?1, ?2)",
            params![content_hash, timestamp],
        )?;
        Ok(())
    }
}

impl MemoryType {
    fn memory_type_name(&self) -> &str {
        match self {
            MemoryType::Working => "working",
            MemoryType::Episodic => "episodic",
            MemoryType::Semantic => "semantic",
            MemoryType::Procedural => "procedural",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_entry(memory_type: MemoryType, content: &str) -> MemoryEntry {
        MemoryEntry::new(memory_type, content)
    }

    #[test]
    fn test_store_and_load() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = MemoryStorage::new(db_path.to_str().unwrap()).unwrap();

        let entry = create_test_entry(MemoryType::Working, "test memory content");
        storage.store_entry(&entry).unwrap();

        let loaded = storage.load_entries_by_type(MemoryType::Working).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "test memory content");
    }

    #[test]
    fn test_stats() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("stats.db");
        let storage = MemoryStorage::new(db_path.to_str().unwrap()).unwrap();

        storage.store_entry(&create_test_entry(MemoryType::Working, "w1")).unwrap();
        storage.store_entry(&create_test_entry(MemoryType::Working, "w2")).unwrap();
        storage.store_entry(&create_test_entry(MemoryType::Episodic, "e1")).unwrap();
        storage.store_entry(&create_test_entry(MemoryType::Semantic, "s1")).unwrap();

        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.working_count, 2);
        assert_eq!(stats.episodic_count, 1);
        assert_eq!(stats.semantic_count, 1);
        assert_eq!(stats.procedural_count, 0);
    }

    #[test]
    fn test_clear_type() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("clear.db");
        let storage = MemoryStorage::new(db_path.to_str().unwrap()).unwrap();

        storage.store_entry(&create_test_entry(MemoryType::Working, "w1")).unwrap();
        storage.store_entry(&create_test_entry(MemoryType::Episodic, "e1")).unwrap();

        let deleted = storage.clear_type(MemoryType::Working).unwrap();
        assert_eq!(deleted, 1);

        let working = storage.load_entries_by_type(MemoryType::Working).unwrap();
        assert!(working.is_empty());
        let episodic = storage.load_entries_by_type(MemoryType::Episodic).unwrap();
        assert_eq!(episodic.len(), 1);
    }
}
