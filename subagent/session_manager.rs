use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Session Manager - tracks and summarizes agent sessions
/// 
/// Automatically creates session summaries when:
/// - Session exceeds token budget
/// - Session ends normally
/// - User requests a summary
pub struct SessionManager {
    conn: std::sync::Mutex<Connection>,
}

impl SessionManager {
    pub fn new(data_dir: &str) -> Result<Self> {
        let db_path = Path::new(data_dir).join("session_manager.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                start_time TEXT NOT NULL,
                end_time TEXT,
                token_count INTEGER NOT NULL DEFAULT 0,
                message_count INTEGER NOT NULL DEFAULT 0,
                summary TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                metadata TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS session_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                content TEXT NOT NULL,
                token_estimate INTEGER NOT NULL DEFAULT 0,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_session_events ON session_events(session_id, timestamp);"
        )?;

        Ok(Self { conn: std::sync::Mutex::new(conn) })
    }

    /// Start a new session
    pub fn start_session(&self, id: &str, metadata: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, start_time, token_count, message_count, status, metadata)
             VALUES (?1, ?2, 0, 0, 'active', ?3)",
            params![id, chrono::Utc::now().to_rfc3339(), metadata],
        )?;
        Ok(())
    }

    /// Log an event in the current session
    pub fn log_event(&self, session_id: &str, event_type: &str, content: &str) -> Result<()> {
        let token_estimate = content.len() / 4; // Rough token estimate
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_events (session_id, event_type, content, token_estimate, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id,
                event_type,
                content,
                token_estimate as i64,
                chrono::Utc::now().to_rfc3339(),
            ],
        )?;

        // Update session stats
        conn.execute(
            "UPDATE sessions SET token_count = token_count + ?1, message_count = message_count + 1
             WHERE id = ?2",
            params![token_estimate as i64, session_id],
        )?;

        Ok(())
    }

    /// End a session and auto-generate summary
    pub async fn end_session(&self, session_id: &str) -> Result<SessionSummary> {
        // Generate summary from session events
        let summary = self.generate_summary(session_id)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET end_time = ?1, summary = ?2, status = 'completed' WHERE id = ?3",
            params![chrono::Utc::now().to_rfc3339(), summary.summary, session_id],
        )?;

        Ok(summary)
    }

    /// Summarize a session (generate or retrieve existing)
    pub async fn summarize(&self, session_id: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();

        // Check if summary already exists
        let existing = conn.query_row(
            "SELECT summary FROM sessions WHERE id = ?1 AND summary IS NOT NULL",
            params![session_id],
            |row| row.get::<_, String>(0),
        );

        if let Ok(summary) = existing {
            return Ok(summary);
        }
        drop(conn);

        // Generate new summary
        let summary = self.generate_summary(session_id)?;
        Ok(summary.summary)
    }

    /// Generate a new summary from session events
    fn generate_summary(&self, session_id: &str) -> Result<SessionSummary> {
        let conn = self.conn.lock().unwrap();

        // Get session info
        let session_info = conn.query_row(
            "SELECT id, start_time, token_count, message_count, metadata FROM sessions WHERE id = ?1",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as usize,
                    row.get::<_, i64>(3)? as usize,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;

        let (id, start_time, token_count, message_count, _metadata) = session_info;

        // Get recent events summary (limit to last 50)
        let mut stmt = conn.prepare(
            "SELECT event_type, content, timestamp FROM session_events 
             WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT 50"
        )?;

        let events: Vec<(String, String, String)> = stmt
            .query_map(params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Count event types
        let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (event_type, _, _) in &events {
            *type_counts.entry(event_type.clone()).or_insert(0) += 1;
        }

        // Find key actions (tool calls, user requests)
        let key_actions: Vec<String> = events
            .iter()
            .filter(|(t, _, _)| t == "tool_call" || t == "user_request")
            .take(5)
            .map(|(_, c, _)| {
                if c.len() > 80 {
                    format!("{}...", &c[..77])
                } else {
                    c.clone()
                }
            })
            .collect();

        let duration = match chrono::DateTime::parse_from_rfc3339(&start_time) {
            Ok(start) => {
                let now = chrono::Utc::now();
                let dur = now.signed_duration_since(start.with_timezone(&chrono::Utc));
                format!("{}m", dur.num_minutes())
            }
            Err(_) => "unknown".to_string(),
        };

        let summary_text = format!(
            "Session {}: {} messages, ~{}k tokens, duration: {}\n\
             Event breakdown: {}\n\
             Key actions: {}\n\
             Latest event: {}",
            &id[..8],
            message_count,
            token_count / 1000,
            duration,
            type_counts.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", "),
            key_actions.join("; "),
            events.first().map(|(_, c, _)| {
                if c.len() > 60 { format!("{}...", &c[..57]) } else { c.clone() }
            }).unwrap_or_default(),
        );

        Ok(SessionSummary {
            session_id: id,
            summary: summary_text,
            message_count,
            token_count,
            event_count: events.len(),
            duration,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Get active sessions
    pub fn get_active_sessions(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id FROM sessions WHERE status = 'active' ORDER BY start_time DESC"
        )?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    /// Get token count for a session
    pub fn get_session_token_count(&self, session_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.query_row(
            "SELECT token_count FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count as usize)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub summary: String,
    pub message_count: usize,
    pub token_count: usize,
    pub event_count: usize,
    pub duration: String,
    pub generated_at: String,
}
