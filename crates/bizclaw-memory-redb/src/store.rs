//! Redb-based key-value store implementation

use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{BatchOperation, DataType, KeyPrefix, ScanOptions, ScanOrder, StorageStats, StoredValue};

const DATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("data");

pub struct RedbStore {
    db: Arc<RwLock<Database>>,
    path: String,
}

impl RedbStore {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_string_lossy().to_string();
        
        if let Some(parent) = Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let db = Database::create(&path)?;
        
        {
            let write_txn = db.begin_write()?;
            write_txn.open_table(DATA_TABLE)?;
            write_txn.commit()?;
        }
        
        info!("RedbStore initialized at {}", path);
        
        Ok(Self {
            db: Arc::new(RwLock::new(db)),
            path,
        })
    }

    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::new(path).await
    }

    pub async fn get(&self, key: &str) -> Result<Option<StoredValue>> {
        let db = self.db.read().await;
        let read_txn = db.begin_read()?;
        
        let table = read_txn.open_table(DATA_TABLE)?;
        
        if let Some(value) = table.get(key)? {
            let bytes = value.value();
            let stored: StoredValue = serde_json::from_slice(bytes)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize: {}", e))?;
            return Ok(Some(stored));
        }
        
        Ok(None)
    }

    pub async fn get_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        if let Some(stored) = self.get(key).await? {
            let value: T = serde_json::from_slice(&stored.value)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub async fn put(&self, key: &str, value: impl serde::Serialize) -> Result<()> {
        let stored = StoredValue::new(key.to_string(), serde_json::to_vec(&value)?);
        self.put_raw(key, stored).await
    }

    pub async fn put_raw(&self, key: &str, mut stored: StoredValue) -> Result<()> {
        let db = self.db.read().await;
        let mut write_txn = db.begin_write()?;
        
        stored.updated_at = chrono::Utc::now();
        
        let bytes = serde_json::to_vec(&stored)?;
        
        {
            let mut table = write_txn.open_table(DATA_TABLE)?;
            table.insert(key, bytes.as_slice())?;
        }
        
        write_txn.commit()?;
        
        debug!("PUT {} (size={} bytes)", key, bytes.len());
        Ok(())
    }

    pub async fn set_with_ttl(&self, key: &str, value: impl serde::Serialize, ttl_seconds: u64) -> Result<()> {
        let mut stored = StoredValue::new(key.to_string(), serde_json::to_vec(&value)?);
        stored.ttl_seconds = Some(ttl_seconds);
        self.put_raw(key, stored).await
    }

    pub async fn delete(&self, key: &str) -> Result<bool> {
        let db = self.db.read().await;
        let mut write_txn = db.begin_write()?;
        let mut existed = false;
        
        {
            let mut table = write_txn.open_table(DATA_TABLE)?;
            if table.remove(key)?.is_some() {
                existed = true;
            }
        }
        
        write_txn.commit()?;
        
        if existed {
            debug!("DELETE {}", key);
        }
        
        Ok(existed)
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let db = self.db.read().await;
        let read_txn = db.begin_read()?;
        
        let table = read_txn.open_table(DATA_TABLE)?;
        Ok(table.get(key)?.is_some())
    }

    pub async fn scan(&self, options: ScanOptions) -> Result<Vec<(String, StoredValue)>> {
        let db = self.db.read().await;
        let read_txn = db.begin_read()?;
        
        let table = read_txn.open_table(DATA_TABLE)?;
        let mut results = Vec::new();
        
        let limit = options.limit.unwrap_or(100);
        let prefix = options.prefix.as_deref();
        
        let iter = match options.order {
            ScanOrder::Forward => Box::new(table.iter()?) as Box<dyn Iterator<Item = _>>,
            ScanOrder::Backward => Box::new(table.iter()?.rev()),
        };
        
        for item in iter {
            let (key, value) = match item {
                Ok(i) => i,
                Err(_) => continue,
            };
            let key_str = key.value().to_string();
            
            if let Some(p) = prefix {
                if !key_str.starts_with(p) {
                    continue;
                }
            }
            
            let stored: StoredValue = match serde_json::from_slice(value.value()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            if !options.include_expired && stored.is_expired() {
                continue;
            }
            
            results.push((key_str, stored));
            
            if results.len() >= limit {
                break;
            }
        }
        
        Ok(results)
    }

    pub async fn scan_prefix(&self, prefix: &str, limit: Option<usize>) -> Result<Vec<String>> {
        let options = ScanOptions {
            prefix: Some(prefix.to_string()),
            limit,
            ..Default::default()
        };
        
        let results = self.scan(options).await?;
        Ok(results.into_iter().map(|(k, _)| k).collect())
    }

    pub async fn batch(&self, ops: BatchOperation) -> Result<()> {
        if ops.is_empty() {
            return Ok(());
        }
        
        let db = self.db.read().await;
        let mut write_txn = db.begin_write()?;
        
        {
            let mut table = write_txn.open_table(DATA_TABLE)?;
            
            for (key, value) in &ops.puts {
                let stored = StoredValue::new(key.clone(), value.clone());
                let bytes = serde_json::to_vec(&stored)?;
                table.insert(key.as_str(), bytes.as_slice())?;
            }
            
            for key in &ops.deletes {
                table.remove(key.as_str())?;
            }
        }
        
        write_txn.commit()?;
        
        debug!("BATCH: {} puts, {} deletes", ops.puts.len(), ops.deletes.len());
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<StorageStats> {
        let db = self.db.read().await;
        let read_txn = db.begin_read()?;
        
        let table = read_txn.open_table(DATA_TABLE)?;
        
        let mut total_keys = 0u64;
        let mut total_size_bytes = 0u64;
        let mut by_type: Vec<(String, u64)> = Vec::new();
        
        for item in table.iter()? {
            let (_, value) = item?;
            total_keys += 1;
            total_size_bytes += value.value().len() as u64;
        }
        
        Ok(StorageStats {
            total_keys,
            total_size_bytes,
            by_type,
            oldest_key: None,
            newest_key: None,
            compaction_candidates: 0,
        })
    }

    pub async fn list_prefixes(&self) -> Result<Vec<KeyPrefix>> {
        Ok(Vec::new())
    }

    pub async fn clear_prefix(&self, prefix: &str) -> Result<u64> {
        let keys = self.scan_prefix(prefix, None).await?;
        let count = keys.len() as u64;
        
        let db = self.db.read().await;
        let mut write_txn = db.begin_write()?;
        
        {
            let mut table = write_txn.open_table(DATA_TABLE)?;
            for key in keys {
                table.remove(key.as_str())?;
            }
        }
        
        write_txn.commit()?;
        
        info!("CLEAR {}: removed {} keys", prefix, count);
        Ok(count)
    }

    pub async fn cleanup_expired(&self) -> Result<u64> {
        Ok(0)
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub async fn compact(&self) -> Result<()> {
        warn!("Redb compaction - file will be compacted on next open");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_basic_operations() {
        let dir = tempdir().unwrap();
        let store = RedbStore::new(dir.path().join("test.db")).await.unwrap();
        
        store.put("test:key1", "value1").await.unwrap();
        let val: Option<String> = store.get_value("test:key1").await.unwrap();
        assert_eq!(val, Some("value1".to_string()));
        
        store.delete("test:key1").await.unwrap();
        let val: Option<String> = store.get_value("test:key1").await.unwrap();
        assert_eq!(val, None);
    }
    
    #[tokio::test]
    async fn test_batch_operations() {
        let dir = tempdir().unwrap();
        let store = RedbStore::new(dir.path().join("test.db")).await.unwrap();
        
        let mut ops = BatchOperation::new();
        ops.put("batch:1".to_string(), b"v1".to_vec());
        ops.put("batch:2".to_string(), b"v2".to_vec());
        
        store.batch(ops).await.unwrap();
        
        let val1 = store.get("batch:1").await.unwrap();
        assert!(val1.is_some());
    }
}
