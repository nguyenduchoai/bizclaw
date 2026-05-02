//! Vector index with embedding generation

use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::hnsw::HnswIndex;
use crate::{BatchResult, DistanceMetric, DocumentMetadata, SearchOptions, SearchResult, VectorDocument, IndexStats};

pub struct VectorIndex {
    index: Arc<RwLock<HnswIndex>>,
    dimension: usize,
    path: String,
}

impl VectorIndex {
    pub fn new<P: AsRef<Path>>(path: P, dimension: usize) -> Self {
        let path_str = path.as_ref().to_string_lossy().to_string();
        
        // Try to load existing index
        let index = if Path::new(&path_str).exists() {
            HnswIndex::load(Path::new(&path_str)).unwrap_or_else(|_| {
                HnswIndex::new(dimension, 16, 200)
            })
        } else {
            HnswIndex::new(dimension, 16, 200)
        };
        
        info!("VectorIndex initialized at {} (dimension={}, vectors={})", 
            path_str, dimension, index.len());
        
        Self {
            index: Arc::new(RwLock::new(index)),
            dimension,
            path: path_str,
        }
    }

    pub async fn insert(&self, doc: VectorDocument) -> Result<(), String> {
        let mut index = self.index.write().await;
        index.insert(doc.id, doc.vector)?;
        self.save_internal(&index).await
    }

    pub async fn insert_batch(&self, docs: Vec<VectorDocument>) -> BatchResult {
        let mut index = self.index.write().await;
        let mut result = BatchResult {
            indexed: 0,
            failed: 0,
            errors: Vec::new(),
        };
        
        for doc in docs {
            match index.insert(doc.id, doc.vector) {
                Ok(_) => result.indexed += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(e);
                }
            }
        }
        
        if result.failed == 0 {
            let _ = self.save_internal(&index).await;
        }
        
        debug!("BATCH insert: {} indexed, {} failed", result.indexed, result.failed);
        result
    }

    pub async fn upsert(&self, doc: VectorDocument) -> Result<(), String> {
        let mut index = self.index.write().await;
        
        // Remove existing if present
        let _ = index.remove(&doc.id);
        
        // Insert new
        index.insert(doc.id, doc.vector)?;
        self.save_internal(&index).await
    }

    pub async fn remove(&self, id: &str) -> bool {
        let mut index = self.index.write().await;
        let removed = index.remove(id);
        
        if removed {
            let _ = self.save_internal(&index).await;
        }
        
        removed
    }

    pub async fn search(
        &self,
        vector: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>, String> {
        if vector.len() != self.dimension {
            return Err(format!(
                "Query dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            ));
        }
        
        let index = self.index.read().await;
        let results = index.search(&vector, options.limit, options.ef_construction);
        
        Ok(results
            .into_iter()
            .map(|(idx, score)| SearchResult {
                id: index.ids.get(idx).cloned().unwrap_or_default(),
                score,
                metadata: DocumentMetadata {
                    doc_type: "memory".to_string(),
                    content: String::new(),
                    title: None,
                    tags: Vec::new(),
                    source: None,
                    created_at: 0,
                    updated_at: 0,
                    access_count: 0,
                },
            })
            .collect())
    }

    pub async fn search_by_text(
        &self,
        _query: &str,
        _embedding: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult>, String> {
        self.search(_embedding, options).await
    }

    pub async fn get(&self, id: &str) -> Option<VectorDocument> {
        let index = self.index.read().await;
        index.get_vector(id).map(|vector| VectorDocument {
            id: id.to_string(),
            vector: vector.clone(),
            metadata: DocumentMetadata {
                doc_type: "memory".to_string(),
                content: String::new(),
                title: None,
                tags: Vec::new(),
                source: None,
                created_at: 0,
                updated_at: 0,
                access_count: 0,
            },
        })
    }

    pub async fn count(&self) -> usize {
        self.index.read().await.len()
    }

    pub async fn dimension(&self) -> usize {
        self.dimension
    }

    pub async fn clear(&self) {
        let mut index = self.index.write().await;
        index.clear();
        let _ = self.save_internal(&index).await;
    }

    pub async fn save(&self) -> Result<(), String> {
        let index = self.index.read().await;
        self.save_internal(&index).await
    }

    async fn save_internal(&self, index: &HnswIndex) -> Result<(), String> {
        index.save(std::path::Path::new(&self.path))
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub async fn stats(&self) -> IndexStats {
        let index = self.index.read().await;
        crate::IndexStats {
            dimension: index.dimension(),
            total_vectors: index.len(),
            max_level: index.max_level,
            metric: DistanceMetric::Cosine,
            size_bytes: 0, // Calculate if needed
        }
    }

    pub async fn find_similar(&self, id: &str, k: usize) -> Vec<(String, f32)> {
        let index = self.index.read().await;
        index.search_by_id(id, k, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_insert_and_search() {
        let dir = tempdir().unwrap();
        let index = VectorIndex::new(dir.path().join("vectors.json"), 4);
        
        let doc = VectorDocument::new(
            "doc1".to_string(),
            vec![0.1, 0.2, 0.3, 0.4],
            "test content".to_string(),
        );
        
        index.insert(doc).await.unwrap();
        
        let results = index
            .search(vec![0.1, 0.2, 0.3, 0.4], SearchOptions::default())
            .await
            .unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
    }

    #[tokio::test]
    async fn test_batch_insert() {
        let dir = tempdir().unwrap();
        let index = VectorIndex::new(dir.path().join("vectors.json"), 4);
        
        let docs = vec![
            VectorDocument::new("a".to_string(), vec![0.1, 0.2, 0.3, 0.4], "doc a".to_string()),
            VectorDocument::new("b".to_string(), vec![0.5, 0.6, 0.7, 0.8], "doc b".to_string()),
            VectorDocument::new("c".to_string(), vec![0.9, 0.8, 0.7, 0.6], "doc c".to_string()),
        ];
        
        let result = index.insert_batch(docs).await;
        
        assert_eq!(result.indexed, 3);
        assert_eq!(result.failed, 0);
        assert_eq!(index.count().await, 3);
    }

    #[tokio::test]
    async fn test_upsert() {
        let dir = tempdir().unwrap();
        let index = VectorIndex::new(dir.path().join("vectors.json"), 2);
        
        // Insert first version
        let doc1 = VectorDocument::new(
            "doc1".to_string(),
            vec![1.0, 2.0],
            "original".to_string(),
        );
        index.insert(doc1).await.unwrap();
        
        // Upsert with new version
        let doc2 = VectorDocument::new(
            "doc1".to_string(),
            vec![3.0, 4.0],
            "updated".to_string(),
        );
        index.upsert(doc2).await.unwrap();
        
        // Should still have only one document
        assert_eq!(index.count().await, 1);
    }
}
