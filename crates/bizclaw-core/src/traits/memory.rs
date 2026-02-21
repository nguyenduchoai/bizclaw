//! Memory Backend trait — swappable persistence.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A memory entry stored in the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Option<Vec<f32>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Search result from memory.
#[derive(Debug, Clone)]
pub struct MemorySearchResult {
    pub entry: MemoryEntry,
    pub score: f32,
}

/// Memory Backend trait — every persistence layer implements this.
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Backend identifier.
    fn name(&self) -> &str;

    /// Save a memory entry.
    async fn save(&self, entry: MemoryEntry) -> Result<()>;

    /// Search memories by text query (hybrid: keyword + vector).
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>>;

    /// Retrieve a specific memory by ID.
    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>>;

    /// Delete a memory entry.
    async fn delete(&self, id: &str) -> Result<()>;

    /// List all memories (with optional limit).
    async fn list(&self, limit: Option<usize>) -> Result<Vec<MemoryEntry>>;

    /// Clear all memories.
    async fn clear(&self) -> Result<()>;
}
