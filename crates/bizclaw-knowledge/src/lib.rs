//! # BizClaw Knowledge Base
//!
//! Ultra-lightweight personal RAG (Retrieval-Augmented Generation).
//! Designed for 512MB RAM devices — no vector DB, no embeddings.
//!
//! ## Design
//! - **SQLite FTS5** for full-text search (built-in, zero setup)
//! - **BM25 scoring** — relevance ranking without embeddings
//! - **Chunking** — split documents into ~500 char chunks
//! - **File-based** — documents stored as-is, index in SQLite
//! - RAM: ~2MB for 1000 document chunks
//!
//! ## How it works
//! ```text
//! User: "Chính sách làm việc từ xa ra sao?"
//!   ↓
//! Knowledge.search("chính sách làm việc từ xa")
//!   ↓ FTS5 + BM25
//! Top 3 chunks from uploaded documents
//!   ↓
//! Injected into Agent system prompt as context
//!   ↓
//! Agent responds with grounded answer
//! ```

pub mod chunker;
pub mod search;
pub mod store;

#[cfg(feature = "pdf")]
pub mod pdf;

pub use search::SearchResult;
pub use store::KnowledgeStore;
