use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

use super::types::{ErrorEntry, ErrorType};

/// SQLite-based storage for error memory entries
pub struct ErrorStorage {
    conn: std::sync::Mutex<Connection>,
}

impl ErrorStorage {
    pub fn new(data_dir: &str) -> Result<Self> {
        let db_path = Path::new(data_dir).join("error_memory.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // Create tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS error_entries (
                id TEXT PRIMARY KEY,
                error_type TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                context TEXT NOT NULL,
                solution TEXT NOT NULL DEFAULT '',
                code_snippet TEXT,
                stack_trace TEXT,
                timestamp TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                frequency INTEGER NOT NULL DEFAULT 1,
                resolved INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]'
            );

            CREATE INDEX IF NOT EXISTS idx_error_type ON error_entries(error_type);
            CREATE INDEX IF NOT EXISTS idx_frequency ON error_entries(frequency DESC);
            CREATE INDEX IF NOT EXISTS idx_last_seen ON error_entries(last_seen DESC);
            CREATE INDEX IF NOT EXISTS idx_resolved ON error_entries(resolved);

            CREATE TABLE IF NOT EXISTS error_relations (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relation_type TEXT NOT NULL DEFAULT 'related',
                PRIMARY KEY (source_id, target_id),
                FOREIGN KEY (source_id) REFERENCES error_entries(id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES error_entries(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS error_tags (
                entry_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (entry_id, tag),
                FOREIGN KEY (entry_id) REFERENCES error_entries(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_tag ON error_tags(tag);"
        )?;

        Ok(Self { conn: std::sync::Mutex::new(conn) })
    }

    /// Store a new error entry
    pub fn store(&self, entry: &ErrorEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tags_json = serde_json::to_string(&entry.tags)?;
        let resolved_int = if entry.resolved { 1 } else { 0 };

        conn.execute(
            "INSERT INTO error_entries (id, error_type, title, description, context, solution, code_snippet, stack_trace, timestamp, last_seen, frequency, resolved, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                entry.id,
                entry.error_type.to_string(),
                entry.title,
                entry.description,
                entry.context,
                entry.solution,
                entry.code_snippet,
                entry.stack_trace,
                entry.timestamp,
                entry.last_seen,
                entry.frequency,
                resolved_int,
                tags_json,
            ],
        )?;

        // Store tags in the tags table
        for tag in &entry.tags {
            conn.execute(
                "INSERT OR IGNORE INTO error_tags (entry_id, tag) VALUES (?1, ?2)",
                params![entry.id, tag],
            )?;
        }

        Ok(())
    }

    /// Update an existing entry
    pub fn update(&self, entry: &ErrorEntry) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let resolved_int = if entry.resolved { 1 } else { 0 };

        conn.execute(
            "UPDATE error_entries SET error_type=?1, title=?2, description=?3, context=?4, 
             solution=?5, code_snippet=?6, stack_trace=?7, last_seen=?8, frequency=?9, resolved=?10
             WHERE id=?11",
            params![
                entry.error_type.to_string(),
                entry.title,
                entry.description,
                entry.context,
                entry.solution,
                entry.code_snippet,
                entry.stack_trace,
                entry.last_seen,
                entry.frequency,
                resolved_int,
                entry.id,
            ],
        )?;
        Ok(())
    }

    /// Get entry by ID
    pub fn get_by_id(&self, id: &str) -> Result<Option<ErrorEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, error_type, title, description, context, solution, code_snippet, 
             stack_trace, timestamp, last_seen, frequency, resolved, tags
             FROM error_entries WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let tags_str: String = row.get(12)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(ErrorEntry {
                id: row.get(0)?,
                error_type: parse_error_type(&row.get::<_, String>(1)?),
                title: row.get(2)?,
                description: row.get(3)?,
                context: row.get(4)?,
                solution: row.get(5)?,
                code_snippet: row.get(6)?,
                stack_trace: row.get(7)?,
                timestamp: row.get(8)?,
                last_seen: row.get(9)?,
                frequency: row.get::<_, i32>(10)? as u32,
                resolved: row.get::<_, i32>(11)? != 0,
                related_errors: Vec::new(),
                tags,
            })
        })?;

        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            _ => Ok(None),
        }
    }

    /// Get the most frequent errors
    pub fn get_most_frequent(&self, limit: usize) -> Result<Vec<ErrorEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, error_type, title, description, context, solution, code_snippet,
             stack_trace, timestamp, last_seen, frequency, resolved, tags
             FROM error_entries ORDER BY frequency DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit as i64], Self::map_row)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Get the most recent errors
    pub fn get_recent(&self, limit: usize) -> Result<Vec<ErrorEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, error_type, title, description, context, solution, code_snippet,
             stack_trace, timestamp, last_seen, frequency, resolved, tags
             FROM error_entries ORDER BY last_seen DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit as i64], Self::map_row)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Get all entries
    pub fn get_all(&self) -> Result<Vec<ErrorEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, error_type, title, description, context, solution, code_snippet,
             stack_trace, timestamp, last_seen, frequency, resolved, tags
             FROM error_entries ORDER BY last_seen DESC"
        )?;

        let rows = stmt.query_map([], Self::map_row)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    /// Delete an entry
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM error_relations WHERE source_id=?1 OR target_id=?1", params![id])?;
        conn.execute("DELETE FROM error_tags WHERE entry_id=?1", params![id])?;
        conn.execute("DELETE FROM error_entries WHERE id=?1", params![id])?;
        Ok(())
    }

    /// Add relation between two errors
    pub fn add_relation(&self, source_id: &str, target_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO error_relations (source_id, target_id) VALUES (?1, ?2)",
            params![source_id, target_id],
        )?;
        Ok(())
    }

    // Helper to map a SQLite row to ErrorEntry
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ErrorEntry> {
        let tags_str: String = row.get(12)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(ErrorEntry {
            id: row.get(0)?,
            error_type: parse_error_type(&row.get::<_, String>(1)?),
            title: row.get(2)?,
            description: row.get(3)?,
            context: row.get(4)?,
            solution: row.get(5)?,
            code_snippet: row.get(6)?,
            stack_trace: row.get(7)?,
            timestamp: row.get(8)?,
            last_seen: row.get(9)?,
            frequency: row.get::<_, i32>(10)? as u32,
            resolved: row.get::<_, i32>(11)? != 0,
            related_errors: Vec::new(),
            tags,
        })
    }
}

fn parse_error_type(s: &str) -> ErrorType {
    match s {
        "compile" => ErrorType::Compile,
        "runtime" => ErrorType::Runtime,
        "network" => ErrorType::Network,
        "logic" => ErrorType::Logic,
        "permission" => ErrorType::Permission,
        "syntax" => ErrorType::Syntax,
        "dependency" => ErrorType::Dependency,
        "tool_execution" => ErrorType::ToolExecution,
        "llm" => ErrorType::Llm,
        _ => ErrorType::Unknown,
    }
}
