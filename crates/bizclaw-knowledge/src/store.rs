//! Knowledge store — SQLite FTS5 for fast full-text search.
//! No vector DB, no embeddings — just BM25 relevance scoring.
//! This is intentionally lightweight for 512MB RAM devices.

use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

use crate::chunker;
use crate::search::SearchResult;

/// Knowledge store backed by SQLite FTS5.
pub struct KnowledgeStore {
    conn: Connection,
}

impl KnowledgeStore {
    /// Open or create a knowledge base at the given path.
    pub fn open(path: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).ok();
        let conn = Connection::open(path).map_err(|e| format!("DB error: {e}"))?;

        // Create tables
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                source TEXT DEFAULT '',
                created_at TEXT DEFAULT (datetime('now')),
                chunk_count INTEGER DEFAULT 0
            );

            -- FTS5 virtual table for full-text search with BM25
            CREATE VIRTUAL TABLE IF NOT EXISTS chunks USING fts5(
                doc_id,
                chunk_idx,
                content,
                tokenize='unicode61'
            );

            -- Metadata for quick stats
            CREATE TABLE IF NOT EXISTS kb_meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );
        ",
        )
        .map_err(|e| format!("Schema error: {e}"))?;

        tracing::debug!("📚 Knowledge store opened: {}", path.display());
        Ok(Self { conn })
    }

    /// Default knowledge base path.
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".bizclaw").join("knowledge.db")
    }

    /// Add a document to the knowledge base.
    /// Automatically chunks and indexes the content.
    pub fn add_document(&self, name: &str, content: &str, source: &str) -> Result<usize, String> {
        // Extract text based on file extension
        let text = chunker::extract_text(content, name);

        // Chunk the text
        let chunks = chunker::chunk_text(&text, 500);
        let chunk_count = chunks.len();

        // Insert document record
        self.conn
            .execute(
                "INSERT INTO documents (name, source, chunk_count) VALUES (?1, ?2, ?3)",
                params![name, source, chunk_count as i64],
            )
            .map_err(|e| format!("Insert doc error: {e}"))?;

        let doc_id = self.conn.last_insert_rowid();

        // Index chunks
        for (idx, chunk) in chunks.iter().enumerate() {
            self.conn
                .execute(
                    "INSERT INTO chunks (doc_id, chunk_idx, content) VALUES (?1, ?2, ?3)",
                    params![doc_id.to_string(), idx.to_string(), chunk],
                )
                .map_err(|e| format!("Insert chunk error: {e}"))?;
        }

        tracing::info!("📄 Added '{}' → {} chunks indexed", name, chunk_count);
        Ok(chunk_count)
    }

    /// Search the knowledge base using BM25 ranking.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let limit = limit.min(10); // Max 10 results

        // Clean query for FTS5
        let clean_query = query
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>();

        if clean_query.trim().is_empty() {
            return Vec::new();
        }

        // FTS5 search with BM25 scoring
        let mut stmt = match self.conn.prepare(
            "SELECT c.doc_id, c.chunk_idx, c.content, d.name, bm25(chunks) as score
             FROM chunks c
             JOIN documents d ON d.id = CAST(c.doc_id AS INTEGER)
             WHERE chunks MATCH ?1
             ORDER BY score
             LIMIT ?2",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("⚠️ Search query error: {e}");
                return Vec::new();
            }
        };

        let results = stmt.query_map(params![clean_query, limit as i64], |row| {
            Ok(SearchResult {
                doc_name: row.get(3)?,
                chunk_idx: row.get::<_, String>(1)?.parse().unwrap_or(0),
                content: row.get(2)?,
                score: row.get(4)?,
            })
        });

        match results {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                tracing::warn!("⚠️ Search error: {e}");
                Vec::new()
            }
        }
    }

    /// List all documents.
    pub fn list_documents(&self) -> Vec<(i64, String, String, i64)> {
        let mut stmt = match self
            .conn
            .prepare("SELECT id, name, source, chunk_count FROM documents ORDER BY id DESC") {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("list_documents prepare error: {e}");
                    return Vec::new();
                }
            };

        stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Remove a document and its chunks.
    pub fn remove_document(&self, doc_id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM chunks WHERE CAST(doc_id AS INTEGER) = ?1",
                params![doc_id],
            )
            .map_err(|e| format!("Delete chunks error: {e}"))?;

        self.conn
            .execute("DELETE FROM documents WHERE id = ?1", params![doc_id])
            .map_err(|e| format!("Delete doc error: {e}"))?;

        Ok(())
    }

    /// Get total stats.
    pub fn stats(&self) -> (usize, usize) {
        let doc_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
            .unwrap_or(0);
        let chunk_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get(0))
            .unwrap_or(0);
        (doc_count as usize, chunk_count as usize)
    }
}
