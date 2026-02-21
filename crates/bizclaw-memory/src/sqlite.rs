//! SQLite memory backend.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::memory::{MemoryBackend, MemoryEntry, MemorySearchResult};
use rusqlite::Connection;
use std::sync::Mutex;

pub struct SqliteMemory {
    conn: Mutex<Connection>,
}

impl SqliteMemory {
    pub fn new() -> Result<Self> {
        let db_path = bizclaw_core::config::BizClawConfig::home_dir().join("memory.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                metadata TEXT DEFAULT '{}',
                embedding BLOB,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"
        ).map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;

        Ok(Self { conn: Mutex::new(conn) })
    }
}

#[async_trait]
impl MemoryBackend for SqliteMemory {
    fn name(&self) -> &str { "sqlite" }

    async fn save(&self, entry: MemoryEntry) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO memories (id, content, metadata, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                entry.id,
                entry.content,
                entry.metadata.to_string(),
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        Ok(())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>> {
        let conn = self.conn.lock().map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, content, metadata, created_at, updated_at FROM memories WHERE content LIKE ?1 LIMIT ?2"
        ).map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;

        let pattern = format!("%{query}%");
        let rows = stmt.query_map(rusqlite::params![pattern, limit], |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                metadata: row.get::<_, String>(2)
                    .map(|s| serde_json::from_str(&s).unwrap_or_default())
                    .unwrap_or_default(),
                embedding: None,
                created_at: row.get::<_, String>(3)
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc)).unwrap_or_default())
                    .unwrap_or_default(),
                updated_at: row.get::<_, String>(4)
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc)).unwrap_or_default())
                    .unwrap_or_default(),
            })
        }).map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;

        let results: Vec<MemorySearchResult> = rows
            .filter_map(|r| r.ok())
            .map(|entry| MemorySearchResult { entry, score: 1.0 })
            .collect();
        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.conn.lock().map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, content, metadata, created_at, updated_at FROM memories WHERE id = ?1"
        ).map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;

        let result = stmt.query_row(rusqlite::params![id], |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                metadata: row.get::<_, String>(2)
                    .map(|s| serde_json::from_str(&s).unwrap_or_default())
                    .unwrap_or_default(),
                embedding: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }).ok();
        Ok(result)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        conn.execute("DELETE FROM memories WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        Ok(())
    }

    async fn list(&self, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let results = self.search("", limit.unwrap_or(100)).await?;
        Ok(results.into_iter().map(|r| r.entry).collect())
    }

    async fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        conn.execute("DELETE FROM memories", [])
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(e.to_string()))?;
        Ok(())
    }
}
