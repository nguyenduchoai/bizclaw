//! # BizClaw Knowledge Base
//!
//! Personal RAG (Retrieval-Augmented Generation) with hybrid search.
//! Works on 512MB RAM devices — no external vector DB needed.
//!
//! ## Design (Privacy First)
//! - **SQLite FTS5** for keyword search (BM25 scoring)
//! - **Vector embeddings** via Ollama or OpenAI (stored as BLOB in SQLite)
//! - **Hybrid search** — keyword (0.3) + vector similarity (0.7)
//! - **Multi-model embedding** — auto-detect + dis_max scoring
//! - **Smart chunking** — heading-aware, 20% overlap for retrieval quality
//! - **Document metadata** — mimetype, owner, file_size for filtering
//! - **Search filters** — narrow results by name, type, owner, score
//! - **Nudges** — proactive suggestions from KB (no LLM needed)
//! - **Interaction Signals** — capture learning signals for continuous improvement
//! - **Folder Watcher** — auto-ingest from `~/.bizclaw/knowledge/`
//! - **DOCX Parser** — extract text from Word documents
//! - **MCP server** — expose KB to Claude Code, Cursor, etc.
//! - **Retry logic** — exponential backoff for embedding API calls
//! - RAM: ~2MB for 1000 document chunks
//!
//! ## Learning Loop
//! ```text
//! User ↔ Agent interaction
//!   ↓
//! Signal Logger captures:
//!   ├── Positive: "Đúng rồi, cảm ơn" → reward +1
//!   ├── Negative: "Sai rồi" → reward -1
//!   ├── Tool success/failure → tool signals
//!   └── Quality Gate APPROVED/REJECTED
//!          ↓
//!   signals.db → export JSONL → future fine-tuning
//! ```

pub mod chunker;
pub mod embeddings;
pub mod graph;
pub mod mcp_server;
pub mod multi_embed;
pub mod nudges;
pub mod search;
pub mod signals;
pub mod store;
pub mod vector_store;
pub mod watcher;

pub use graph::{Entity, EntityType, GraphService, KnowledgeGraph, RelationType, Relationship};

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "docx")]
pub mod docx;

pub use mcp_server::{McpToolCall, McpToolDef, McpToolResponse};
pub use multi_embed::{CombineStrategy, MultiModelEmbedder};
pub use nudges::{Nudge, NudgeConfig, NudgeEngine};
pub use search::{SearchFilter, SearchResult, SearchTelemetry};
pub use signals::{InteractionSignal, SignalLogger, SignalType};
pub use store::{DocumentInfo, KnowledgeStore};
pub use watcher::{FolderWatcher, IngestResult};
