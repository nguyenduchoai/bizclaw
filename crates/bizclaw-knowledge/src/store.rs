//! Knowledge store — SQLite FTS5 for fast full-text search.
//! Enhanced with document metadata (mimetype, owner, size) and filtered search.
//! This is intentionally lightweight for 512MB RAM devices.

use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

use crate::chunker;
use crate::search::{SearchFilter, SearchResult, SearchTelemetry};

/// Detect MIME type from file extension.
fn detect_mimetype(filename: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "md" | "markdown" => "text/markdown",
        "txt" | "text" => "text/plain",
        "json" => "application/json",
        "csv" => "text/csv",
        "toml" => "application/toml",
        "yaml" | "yml" => "application/yaml",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "log" => "text/x-log",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Document metadata returned by list_documents.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentInfo {
    pub id: i64,
    pub name: String,
    pub source: String,
    pub chunk_count: i64,
    pub mimetype: String,
    pub owner: String,
    pub file_size: i64,
    pub created_at: String,
}

/// Knowledge store backed by SQLite FTS5.
pub struct KnowledgeStore {
    conn: Connection,
}

impl KnowledgeStore {
    /// Open or create a knowledge base at the given path.
    pub fn open(path: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).ok();
        let conn = Connection::open(path).map_err(|e| format!("DB error: {e}"))?;

        // Create tables — v2 schema with metadata columns
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

        // Migrate: add metadata columns if they don't exist (safe for existing DBs)
        Self::migrate_add_metadata(&conn);

        tracing::debug!("📚 Knowledge store opened: {}", path.display());
        Ok(Self { conn })
    }

    /// Add metadata columns to the documents table (v2 migration).
    /// Safe to call multiple times — silently ignores "already exists" errors.
    fn migrate_add_metadata(conn: &Connection) {
        let migrations = [
            "ALTER TABLE documents ADD COLUMN mimetype TEXT DEFAULT ''",
            "ALTER TABLE documents ADD COLUMN owner TEXT DEFAULT ''",
            "ALTER TABLE documents ADD COLUMN file_size INTEGER DEFAULT 0",
        ];
        for sql in &migrations {
            // ALTER TABLE ADD COLUMN errors if column already exists — that's OK
            if let Err(e) = conn.execute_batch(sql) {
                let msg = e.to_string();
                if !msg.contains("duplicate column") && !msg.contains("already exists") {
                    tracing::warn!("Migration warning: {msg}");
                }
            }
        }
    }

    /// Default knowledge base path.
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".bizclaw").join("knowledge.db")
    }

    /// Add a document to the knowledge base.
    /// Automatically chunks and indexes the content.
    pub fn add_document(&self, name: &str, content: &str, source: &str) -> Result<usize, String> {
        self.add_document_with_meta(name, content, source, "", 0)
    }

    /// Add a document with full metadata.
    pub fn add_document_with_meta(
        &self,
        name: &str,
        content: &str,
        source: &str,
        owner: &str,
        file_size: usize,
    ) -> Result<usize, String> {
        // Extract text based on file extension
        let text = chunker::extract_text(content, name);

        // Smart chunking that respects document structure
        let chunks = chunker::chunk_text(&text, 500);
        let chunk_count = chunks.len();
        let mimetype = detect_mimetype(name);

        // Insert document record with metadata
        self.conn
            .execute(
                "INSERT INTO documents (name, source, chunk_count, mimetype, owner, file_size) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![name, source, chunk_count as i64, mimetype, owner, file_size as i64],
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

        tracing::info!(
            "📄 Added '{}' ({}) → {} chunks indexed [owner={}, size={}]",
            name,
            mimetype,
            chunk_count,
            owner,
            file_size
        );
        Ok(chunk_count)
    }

    /// Add a PDF document from raw bytes.
    /// Uses pdf_oxide for text extraction with markdown preservation.
    /// Tries markdown extraction first (better RAG quality), falls back to plain text.
    #[cfg(feature = "pdf")]
    pub fn add_pdf_document(&self, name: &str, data: &[u8], source: &str) -> Result<usize, String> {
        self.add_pdf_document_with_meta(name, data, source, "")
    }

    /// Add a PDF document with owner metadata.
    #[cfg(feature = "pdf")]
    pub fn add_pdf_document_with_meta(
        &self,
        name: &str,
        data: &[u8],
        source: &str,
        owner: &str,
    ) -> Result<usize, String> {
        // Try markdown first (preserves headings, tables, layout)
        // Fall back to plain text if markdown fails
        let text = crate::pdf::extract_markdown_from_pdf(data)
            .or_else(|_| crate::pdf::extract_text_from_pdf(data))?;

        let file_size = data.len();
        let chunks = chunker::chunk_text(&text, 500);
        let chunk_count = chunks.len();

        // Insert document record with metadata
        self.conn
            .execute(
                "INSERT INTO documents (name, source, chunk_count, mimetype, owner, file_size) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![name, source, chunk_count as i64, "application/pdf", owner, file_size as i64],
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

        tracing::info!(
            "📄 Added PDF '{}' → {} chunks indexed [owner={}]",
            name,
            chunk_count,
            owner
        );
        Ok(chunk_count)
    }

    /// Search the knowledge base using BM25 ranking.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        self.search_filtered(query, limit, &SearchFilter::default())
    }

    /// Search with filters — the core search method.
    /// Pre-filters at SQL level where possible, post-filters for score threshold.
    pub fn search_filtered(
        &self,
        query: &str,
        limit: usize,
        filter: &SearchFilter,
    ) -> Vec<SearchResult> {
        let limit = limit.min(20); // Max 20 results

        // Clean query for FTS5
        let clean_query = query
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>();

        if clean_query.trim().is_empty() {
            return Vec::new();
        }

        // Build SQL with optional filter conditions
        let (filter_clause, filter_params) = filter.to_sql_conditions();

        // We need to build the SQL dynamically because of variable filter params
        let sql = format!(
            "SELECT c.doc_id, c.chunk_idx, c.content, d.name, bm25(chunks) as score,
                    d.mimetype, d.owner
             FROM chunks c
             JOIN documents d ON d.id = CAST(c.doc_id AS INTEGER)
             WHERE chunks MATCH ?1{filter_clause}
             ORDER BY score
             LIMIT ?2"
        );

        let mut stmt = match self.conn.prepare(&sql) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("⚠️ Search query error: {e}");
                return Vec::new();
            }
        };

        // Build parameter list: [query, ...filter_params, limit]
        // rusqlite doesn't support dynamic params easily, so we use a different approach
        // Use rusqlite::params_from_iter for dynamic parameters
        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        all_params.push(Box::new(clean_query));
        for p in &filter_params {
            all_params.push(Box::new(p.clone()));
        }
        all_params.push(Box::new(limit as i64));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            all_params.iter().map(|p| p.as_ref()).collect();

        let results = stmt.query_map(param_refs.as_slice(), |row| {
            let mimetype: Option<String> = row.get(5).ok();
            let owner: Option<String> = row.get(6).ok();
            Ok(SearchResult {
                doc_name: row.get(3)?,
                chunk_idx: row.get::<_, String>(1)?.parse().unwrap_or(0),
                content: row.get(2)?,
                score: row.get(4)?,
                mimetype: mimetype.filter(|s| !s.is_empty()),
                owner: owner.filter(|s| !s.is_empty()),
            })
        });

        match results {
            Ok(rows) => {
                let mut results: Vec<SearchResult> = rows.filter_map(|r| r.ok()).collect();
                // Apply score threshold post-filter
                if let Some(threshold) = filter.score_threshold {
                    results.retain(|r| r.score.abs() >= threshold);
                }
                results
            }
            Err(e) => {
                tracing::warn!("⚠️ Search error: {e}");
                Vec::new()
            }
        }
    }

    /// List all documents with full metadata.
    pub fn list_documents(&self) -> Vec<DocumentInfo> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, name, source, chunk_count, mimetype, owner, file_size, created_at
             FROM documents ORDER BY id DESC",
        ) {
            Ok(s) => s,
            Err(_) => {
                // Fallback for old schema without metadata columns
                return self.list_documents_legacy();
            }
        };

        stmt.query_map([], |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                chunk_count: row.get(3)?,
                mimetype: row.get::<_, String>(4).unwrap_or_default(),
                owner: row.get::<_, String>(5).unwrap_or_default(),
                file_size: row.get::<_, i64>(6).unwrap_or(0),
                created_at: row.get::<_, String>(7).unwrap_or_default(),
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Legacy list_documents for backward compatibility with old schema.
    fn list_documents_legacy(&self) -> Vec<DocumentInfo> {
        let mut stmt = match self
            .conn
            .prepare("SELECT id, name, source, chunk_count FROM documents ORDER BY id DESC")
        {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("list_documents prepare error: {e}");
                return Vec::new();
            }
        };

        stmt.query_map([], |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                chunk_count: row.get(3)?,
                mimetype: String::new(),
                owner: String::new(),
                file_size: 0,
                created_at: String::new(),
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Remove a document and its chunks.
    pub fn remove_document(&self, doc_id: i64) -> Result<(), String> {
        // Also remove embeddings if vector table exists
        let _ = self.conn.execute(
            "DELETE FROM chunk_embeddings WHERE CAST(doc_id AS INTEGER) = ?1",
            params![doc_id],
        );

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

    /// Remove a document by its name.
    pub fn remove_document_by_name(&self, name: &str) -> Result<(), String> {
        let doc_id: i64 = match self.conn.query_row(
            "SELECT id FROM documents WHERE name = ?1 LIMIT 1",
            params![name],
            |r| r.get(0),
        ) {
            Ok(id) => id,
            Err(_) => return Ok(()), // Not found is fine
        };
        self.remove_document(doc_id)
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

    /// Get detailed stats including metadata breakdown.
    pub fn detailed_stats(&self) -> serde_json::Value {
        let (doc_count, chunk_count) = self.stats();
        let embedded = self.embedded_count();

        // Count by mimetype
        let type_counts: Vec<(String, i64)> = self
            .conn
            .prepare(
                "SELECT COALESCE(mimetype, 'unknown'), COUNT(*) FROM documents GROUP BY mimetype",
            )
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        // Total file size
        let total_size: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(file_size), 0) FROM documents",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        serde_json::json!({
            "documents": doc_count,
            "chunks": chunk_count,
            "embedded_chunks": embedded,
            "embedding_coverage": if chunk_count > 0 {
                format!("{:.0}%", (embedded as f64 / chunk_count as f64) * 100.0)
            } else {
                "0%".to_string()
            },
            "total_file_size": total_size,
            "by_type": type_counts.into_iter()
                .map(|(t, c)| serde_json::json!({"type": t, "count": c}))
                .collect::<Vec<_>>(),
        })
    }

    // ── Vector RAG Methods ──────────────────────────────────────────

    /// Enable vector search by creating the embeddings table.
    pub fn enable_vectors(&self) -> Result<(), String> {
        crate::vector_store::ensure_vector_schema(&self.conn)
    }

    /// Hybrid search: keyword (BM25) + vector (cosine similarity).
    /// If query_embedding is None, falls back to keyword-only search.
    pub fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> Vec<SearchResult> {
        self.hybrid_search_filtered(query, query_embedding, limit, &SearchFilter::default())
    }

    /// Hybrid search with filters — the most powerful search method.
    pub fn hybrid_search_filtered(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
        filter: &SearchFilter,
    ) -> Vec<SearchResult> {
        let mut results = crate::vector_store::hybrid_search(
            &self.conn,
            query,
            query_embedding,
            limit,
            0.3, // keyword weight
            0.7, // vector weight
        );

        // Apply filters post-search (brute-force is fine for <10K results)
        if !filter.is_empty() {
            results.retain(|r| filter.matches(r));
        }

        results
    }

    /// Get count of chunks that have embeddings.
    pub fn embedded_count(&self) -> usize {
        crate::vector_store::embedded_chunk_count(&self.conn)
    }

    /// Store embedding for a specific chunk.
    pub fn store_chunk_embedding(
        &self,
        doc_id: &str,
        chunk_idx: &str,
        embedding: &[f32],
    ) -> Result<(), String> {
        crate::vector_store::store_embedding(&self.conn, doc_id, chunk_idx, embedding)
    }

    /// Get all chunk texts that are missing embeddings (for batch embedding).
    pub fn chunks_without_embeddings(&self) -> Vec<(String, String, String)> {
        self.enable_vectors().ok();
        let mut stmt = match self.conn.prepare(
            "SELECT c.doc_id, c.chunk_idx, c.content FROM chunks c
             LEFT JOIN chunk_embeddings ce ON ce.doc_id = c.doc_id AND ce.chunk_idx = c.chunk_idx
             WHERE ce.embedding IS NULL",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    }

    // ── Hybrid Recall Enhancement (Goclaw-inspired) ─────────────────

    /// Enhanced hybrid search with TF-IDF exact-match boost.
    ///
    /// Problem: Vector embeddings can miss domain-specific terms
    /// (Vietnamese product names, technical jargon, acronyms like "BizClaw", "PaaS").
    ///
    /// Solution: After standard hybrid search, boost results that contain
    /// exact query terms with a TF-IDF-like score multiplier.
    /// This is lightweight (O(n*m) where n=results, m=query_terms).
    pub fn hybrid_search_boosted(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
        filter: &SearchFilter,
    ) -> (Vec<SearchResult>, SearchTelemetry) {
        let start = std::time::Instant::now();

        // Step 1: Standard hybrid search (BM25 + vector)
        let mut results = self.hybrid_search_filtered(query, query_embedding, limit * 2, filter);

        let pre_boost_count = results.len();

        // Step 2: TF-IDF exact-match boost
        let query_terms: Vec<String> = query
            .split_whitespace()
            .filter(|t| t.len() >= 2) // Skip single-char terms
            .map(|t| t.to_lowercase())
            .collect();

        if !query_terms.is_empty() {
            let total_docs = results.len().max(1) as f64;

            // Pre-compute document frequency for each term
            let dfs: Vec<f64> = query_terms
                .iter()
                .map(|term| {
                    results
                        .iter()
                        .filter(|r| r.content.to_lowercase().contains(term.as_str()))
                        .count() as f64
                })
                .collect();

            for result in &mut results {
                let content_lower = result.content.to_lowercase();
                let mut boost = 0.0f64;

                for (i, term) in query_terms.iter().enumerate() {
                    // TF: term frequency in this chunk
                    let tf = content_lower.matches(term.as_str()).count() as f64;
                    if tf > 0.0 {
                        // IDF: log(total_docs / (1 + df)) where df = docs containing this term
                        let idf = (total_docs / (1.0 + dfs[i])).ln().max(0.1);
                        boost += tf * idf * 0.1; // Scale factor to blend with existing score
                    }
                }

                // Apply boost — additive for high-match results
                if boost > 0.0 {
                    result.score += boost;
                }
            }

            // Re-sort after boosting
            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        results.truncate(limit);

        let elapsed_ms = start.elapsed().as_millis() as u64;
        let telemetry = SearchTelemetry {
            query_terms: query_terms.len(),
            keyword_results: pre_boost_count,
            final_results: results.len(),
            boost_applied: !query_terms.is_empty(),
            search_ms: elapsed_ms,
            hit: !results.is_empty(),
        };

        tracing::debug!(
            "🔍 Hybrid search: {} terms, {}/{} results, {}ms, {}",
            telemetry.query_terms,
            telemetry.final_results,
            telemetry.keyword_results,
            telemetry.search_ms,
            if telemetry.hit { "HIT" } else { "MISS" }
        );

        (results, telemetry)
    }

    /// Get the underlying connection (for advanced users).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> KnowledgeStore {
        KnowledgeStore::open(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn test_add_and_search() {
        let store = create_test_store();
        let chunks = store
            .add_document(
                "test.md",
                "BizClaw is an AI agent platform for SMEs",
                "test",
            )
            .unwrap();
        assert!(chunks > 0);

        let results = store.search("BizClaw AI", 5);
        assert!(!results.is_empty());
        assert!(results[0].content.contains("BizClaw"));
    }

    #[test]
    fn test_add_with_metadata() {
        let store = create_test_store();
        store
            .add_document_with_meta(
                "policy.pdf",
                "Company policy content",
                "upload",
                "admin",
                1024,
            )
            .unwrap();

        let docs = store.list_documents();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].name, "policy.pdf");
        assert_eq!(docs[0].mimetype, "application/pdf");
        assert_eq!(docs[0].owner, "admin");
        assert_eq!(docs[0].file_size, 1024);
    }

    #[test]
    fn test_search_with_filters() {
        let store = create_test_store();
        store
            .add_document_with_meta(
                "policy.md",
                "Chính sách công ty về nghỉ phép",
                "upload",
                "hr",
                500,
            )
            .unwrap();
        store
            .add_document_with_meta("tech.md", "Kiến trúc hệ thống AI", "upload", "dev", 800)
            .unwrap();

        // Search with owner filter
        let filter = SearchFilter {
            owners: Some(vec!["hr".into()]),
            ..Default::default()
        };
        let results = store.search_filtered("chính sách", 5, &filter);
        // Results should only contain docs from "hr" owner
        for r in &results {
            if let Some(owner) = &r.owner {
                assert_eq!(owner, "hr");
            }
        }
    }

    #[test]
    fn test_detect_mimetype() {
        assert_eq!(detect_mimetype("doc.pdf"), "application/pdf");
        assert_eq!(detect_mimetype("readme.md"), "text/markdown");
        assert_eq!(detect_mimetype("data.csv"), "text/csv");
        assert_eq!(detect_mimetype("unknown"), "application/octet-stream");
    }

    #[test]
    fn test_detailed_stats() {
        let store = create_test_store();
        store.add_document("a.md", "Content A", "test").unwrap();
        store.add_document("b.pdf", "Content B", "test").unwrap();

        let stats = store.detailed_stats();
        assert_eq!(stats["documents"], 2);
    }

    #[test]
    fn test_remove_document() {
        let store = create_test_store();
        store.add_document("test.md", "Content", "test").unwrap();
        let docs = store.list_documents();
        assert_eq!(docs.len(), 1);

        store.remove_document(docs[0].id).unwrap();
        assert_eq!(store.list_documents().len(), 0);
    }

    #[test]
    fn test_boosted_search_hit() {
        let store = create_test_store();
        store
            .add_document(
                "product.md",
                "BizClaw là nền tảng AI agent cho doanh nghiệp SME. BizClaw PaaS giúp tự động hóa.",
                "test",
            )
            .unwrap();
        store
            .add_document("generic.md", "Phần mềm quản lý cho doanh nghiệp", "test")
            .unwrap();

        let (results, telemetry) =
            store.hybrid_search_boosted("BizClaw PaaS", None, 5, &SearchFilter::default());

        assert!(telemetry.hit, "Search should be a HIT");
        assert!(telemetry.boost_applied, "TF-IDF boost should be applied");
        assert!(
            telemetry.query_terms >= 2,
            "Should have at least 2 query terms"
        );
        assert!(!results.is_empty());

        // BizClaw-specific doc should rank higher (has exact term matches)
        if results.len() >= 2 {
            assert!(
                results[0].content.contains("BizClaw"),
                "First result should contain the exact domain term 'BizClaw'"
            );
        }
    }

    #[test]
    fn test_boosted_search_miss() {
        let store = create_test_store();
        store
            .add_document("test.md", "Simple content about weather", "test")
            .unwrap();

        let (results, telemetry) = store.hybrid_search_boosted(
            "blockchain cryptocurrency NFT",
            None,
            5,
            &SearchFilter::default(),
        );

        assert!(!telemetry.hit, "Search should be a MISS");
        assert_eq!(telemetry.final_results, 0);
    }

    #[test]
    fn test_search_telemetry_summary() {
        use crate::search::SearchTelemetry;

        let tele = SearchTelemetry {
            query_terms: 3,
            keyword_results: 10,
            final_results: 5,
            boost_applied: true,
            search_ms: 42,
            hit: true,
        };

        let summary = tele.summary();
        assert!(summary.contains("HIT"));
        assert!(summary.contains("3 terms"));
        assert!(summary.contains("5/10"));
        assert!(summary.contains("42ms"));

        let miss = SearchTelemetry::empty();
        assert!(!miss.hit);
        assert!(miss.summary().contains("MISS"));
    }
}
