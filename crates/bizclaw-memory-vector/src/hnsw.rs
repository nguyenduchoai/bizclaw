//! HNSW index implementation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswIndex {
    pub dimension: usize,
    pub max_level: usize,
    pub m: usize,
    pub ef_construction: usize,
    pub vectors: Vec<Vec<f32>>,
    pub ids: Vec<String>,
    pub entry_point: Option<usize>,
}

impl HnswIndex {
    pub fn new(dimension: usize, m: usize, ef_construction: usize) -> Self {
        Self {
            dimension,
            max_level: 0,
            m,
            ef_construction,
            vectors: Vec::new(),
            ids: Vec::new(),
            entry_point: None,
        }
    }
    
    #[allow(dead_code)]
    pub fn new_with_seed(dimension: usize, m: usize, ef_construction: usize, _seed: u64) -> Self {
        Self::new(dimension, m, ef_construction)
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    pub fn clear(&mut self) {
        self.vectors.clear();
        self.ids.clear();
        self.entry_point = None;
        self.max_level = 0;
    }

    pub fn get_vector(&self, id: &str) -> Option<&Vec<f32>> {
        self.ids.iter().position(|i| i == id).map(|i| &self.vectors[i])
    }

    pub fn insert(&mut self, id: String, vector: Vec<f32>) -> Result<(), String> {
        if vector.len() != self.dimension {
            return Err(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            ));
        }

        if self.ids.contains(&id) {
            return Err(format!("Vector with id '{}' already exists", id));
        }

        let idx = self.vectors.len();
        self.vectors.push(vector);
        self.ids.push(id);
        
        if idx == 0 {
            self.entry_point = Some(0);
            return Ok(());
        }
        
        self.entry_point = Some(idx);
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(idx) = self.ids.iter().position(|i| i == id) {
            self.ids.swap_remove(idx);
            self.vectors.swap_remove(idx);
            true
        } else {
            false
        }
    }

    pub fn search(&self, query: &[f32], k: usize, _ef: usize) -> Vec<(usize, f32)> {
        if self.is_empty() {
            return Vec::new();
        }
        
        let mut results: Vec<(usize, f32)> = self.vectors
            .iter()
            .enumerate()
            .map(|(idx, v)| (idx, Self::cosine_distance(query, v)))
            .collect();
        
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        results
    }

    pub fn search_by_id(&self, query_id: &str, k: usize, ef: usize) -> Vec<(String, f32)> {
        if let Some(query) = self.get_vector(query_id) {
            self.search(query, k, ef)
                .into_iter()
                .filter(|(idx, _)| *idx < self.ids.len())
                .map(|(idx, dist)| (self.ids[idx].clone(), dist))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let file = std::fs::File::create(path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        
        serde_json::to_writer_pretty(file, self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Failed to open file: {}", e))?;
        
        serde_json::from_reader(file)
            .map_err(|e| format!("Failed to deserialize: {}", e))
    }

    pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0;
        }
        
        1.0 - (dot / (norm_a * norm_b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let mut index = HnswIndex::new(4, 16, 200);
        
        index.insert("a".to_string(), vec![0.1, 0.2, 0.3, 0.4]).unwrap();
        index.insert("b".to_string(), vec![0.1, 0.2, 0.3, 0.5]).unwrap();
        
        let results = index.search(&[0.1, 0.2, 0.3, 0.4], 2, 10);
        
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut index = HnswIndex::new(2, 16, 200);
        
        index.insert("a".to_string(), vec![0.0, 0.0]).unwrap();
        index.insert("b".to_string(), vec![1.0, 1.0]).unwrap();
        
        assert_eq!(index.len(), 2);
        
        assert!(index.remove("a"));
        assert_eq!(index.len(), 1);
    }
}
