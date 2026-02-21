//! No-op memory backend — no persistence.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::memory::{MemoryBackend, MemoryEntry, MemorySearchResult};

/// No-op memory backend.
pub struct NoopMemory;

#[async_trait]
impl MemoryBackend for NoopMemory {
    fn name(&self) -> &str { "none" }
    async fn save(&self, _entry: MemoryEntry) -> Result<()> { Ok(()) }
    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<MemorySearchResult>> { Ok(vec![]) }
    async fn get(&self, _id: &str) -> Result<Option<MemoryEntry>> { Ok(None) }
    async fn delete(&self, _id: &str) -> Result<()> { Ok(()) }
    async fn list(&self, _limit: Option<usize>) -> Result<Vec<MemoryEntry>> { Ok(vec![]) }
    async fn clear(&self) -> Result<()> { Ok(()) }
}
