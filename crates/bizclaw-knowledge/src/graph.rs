//! # Knowledge Graph - Semantic Search Engine
//!
//! Graph-based knowledge storage với entity relationships và semantic search.
//!
//! ## Features:
//! - Entity extraction từ text
//! - Relationship tracking giữa entities
//! - Semantic search với embeddings
//! - Path finding giữa entities
//! - Context-aware retrieval

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub properties: HashMap<String, String>,
    pub description: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Product,
    Concept,
    Location,
    Event,
    Document,
    Task,
    Custom(String),
}

impl EntityType {
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Person => "person",
            EntityType::Organization => "organization",
            EntityType::Product => "product",
            EntityType::Concept => "concept",
            EntityType::Location => "location",
            EntityType::Event => "event",
            EntityType::Document => "document",
            EntityType::Task => "task",
            EntityType::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub source: EntityId,
    pub target: EntityId,
    pub relation_type: RelationType,
    pub properties: HashMap<String, String>,
    pub weight: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    WorksFor,
    LocatedIn,
    PartOf,
    RelatedTo,
    DependsOn,
    CreatedBy,
    ManagedBy,
    CollaboratesWith,
    Owns,
    Custom(String),
}

impl RelationType {
    pub fn as_str(&self) -> &str {
        match self {
            RelationType::WorksFor => "works_for",
            RelationType::LocatedIn => "located_in",
            RelationType::PartOf => "part_of",
            RelationType::RelatedTo => "related_to",
            RelationType::DependsOn => "depends_on",
            RelationType::CreatedBy => "created_by",
            RelationType::ManagedBy => "managed_by",
            RelationType::CollaboratesWith => "collaborates_with",
            RelationType::Owns => "owns",
            RelationType::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    pub entity_name: Option<String>,
    pub entity_type: Option<EntityType>,
    pub relation_type: Option<RelationType>,
    pub depth: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
    pub score: f32,
    pub path: Option<Vec<EntityId>>,
}

pub struct KnowledgeGraph {
    entities: RwLock<HashMap<EntityId, Entity>>,
    relationships: RwLock<HashMap<String, Relationship>>,
    adjacency: RwLock<HashMap<EntityId, HashSet<String>>>,
    embeddings_index: RwLock<HashMap<EntityId, Vec<f32>>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: RwLock::new(HashMap::new()),
            relationships: RwLock::new(HashMap::new()),
            adjacency: RwLock::new(HashMap::new()),
            embeddings_index: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_entity(&self, entity: Entity) -> EntityId {
        let id = entity.id.clone();

        if let Some(embedding) = &entity.embedding {
            self.embeddings_index
                .write()
                .await
                .insert(id.clone(), embedding.clone());
        }

        self.entities
            .write()
            .await
            .insert(id.clone(), entity.clone());
        self.adjacency.write().await.entry(id.clone()).or_default();

        id
    }

    pub async fn add_relationship(&self, relationship: Relationship) -> String {
        let id = relationship.id.clone();

        self.adjacency
            .write()
            .await
            .entry(relationship.source.clone())
            .or_default()
            .insert(relationship.id.clone());

        self.adjacency
            .write()
            .await
            .entry(relationship.target.clone())
            .or_default();

        self.relationships
            .write()
            .await
            .insert(id.clone(), relationship);

        id
    }

    pub async fn get_entity(&self, id: &EntityId) -> Option<Entity> {
        self.entities.read().await.get(id).cloned()
    }

    pub async fn find_entities(&self, query: &GraphQuery) -> Vec<Entity> {
        let entities = self.entities.read().await;

        entities
            .values()
            .filter(|e| {
                if let Some(name) = &query.entity_name {
                    if !e.name.to_lowercase().contains(&name.to_lowercase()) {
                        return false;
                    }
                }
                if let Some(entity_type) = &query.entity_type {
                    if &e.entity_type != entity_type {
                        return false;
                    }
                }
                true
            })
            .take(query.limit.unwrap_or(100))
            .cloned()
            .collect()
    }

    pub async fn get_neighbors(&self, id: &EntityId, depth: usize) -> Vec<(Entity, Relationship)> {
        let entities = self.entities.read().await;
        let relationships = self.relationships.read().await;
        let adjacency = self.adjacency.read().await;

        let mut results = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![(id.clone(), 0)];

        while let Some((current_id, current_depth)) = queue.pop() {
            if visited.contains(&current_id) || current_depth > depth {
                continue;
            }
            visited.insert(current_id.clone());

            if let Some(neighbor_ids) = adjacency.get(&current_id) {
                for rel_id in neighbor_ids {
                    if let Some(rel) = relationships.get(rel_id) {
                        if rel.source == *id || rel.target == *id {
                            let neighbor_id = if rel.source == *id {
                                &rel.target
                            } else {
                                &rel.source
                            };
                            if let Some(entity) = entities.get(neighbor_id) {
                                results.push((entity.clone(), rel.clone()));
                                queue.push((neighbor_id.clone(), current_depth + 1));
                            }
                        }
                    }
                }
            }
        }

        results
    }

    pub async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Vec<(Entity, f32)> {
        let entities = self.entities.read().await;
        let embeddings = self.embeddings_index.read().await;

        let mut scores: Vec<(Entity, f32)> = entities
            .values()
            .filter_map(|e| {
                embeddings.get(&e.id).map(|emb| {
                    let similarity = cosine_similarity(query_embedding, emb);
                    (e.clone(), similarity)
                })
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(limit);

        scores
    }

    pub async fn find_path(
        &self,
        source: &EntityId,
        target: &EntityId,
        max_depth: usize,
    ) -> Option<Vec<EntityId>> {
        let adjacency = self.adjacency.read().await;
        let relationships = self.relationships.read().await;

        let mut visited = HashSet::new();
        let mut queue = vec![(source.clone(), vec![source.clone()])];

        while let Some((current_id, mut path)) = queue.pop() {
            if current_id == *target {
                return Some(path);
            }

            if visited.contains(&current_id) || path.len() > max_depth {
                continue;
            }
            visited.insert(current_id.clone());

            if let Some(neighbor_ids) = adjacency.get(&current_id) {
                for rel_id in neighbor_ids {
                    if let Some(rel) = relationships.get(rel_id) {
                        let neighbor = if rel.source == current_id {
                            &rel.target
                        } else {
                            &rel.source
                        };
                        if !visited.contains(neighbor) {
                            let mut new_path = path.clone();
                            new_path.push(neighbor.clone());
                            queue.push((neighbor.clone(), new_path));
                        }
                    }
                }
            }
        }

        None
    }

    pub async fn get_stats(&self) -> GraphStats {
        let entities = self.entities.read().await;
        let relationships = self.relationships.read().await;

        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for entity in entities.values() {
            *type_counts
                .entry(entity.entity_type.as_str().to_string())
                .or_default() += 1;
        }

        let mut relation_counts: HashMap<String, usize> = HashMap::new();
        for rel in relationships.values() {
            *relation_counts
                .entry(rel.relation_type.as_str().to_string())
                .or_default() += 1;
        }

        GraphStats {
            total_entities: entities.len(),
            total_relationships: relationships.len(),
            entity_types: type_counts,
            relation_types: relation_counts,
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_entities: usize,
    pub total_relationships: usize,
    pub entity_types: HashMap<String, usize>,
    pub relation_types: HashMap<String, usize>,
}

pub struct GraphService {
    graph: Arc<KnowledgeGraph>,
}

impl GraphService {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(KnowledgeGraph::new()),
        }
    }

    pub fn graph(&self) -> Arc<KnowledgeGraph> {
        self.graph.clone()
    }

    pub async fn extract_entities(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            let word = word.trim_matches(|c: char| !c.is_alphanumeric());
            if word.len() > 2 {
                let capitalized: String = word
                    .chars()
                    .enumerate()
                    .map(|(j, c)| {
                        if j == 0 {
                            c.to_uppercase().to_string()
                        } else {
                            c.to_string()
                        }
                    })
                    .collect();

                entities.push(Entity {
                    id: EntityId(format!("entity_{}", uuid::Uuid::new_v4())),
                    name: capitalized,
                    entity_type: EntityType::Concept,
                    properties: HashMap::new(),
                    description: None,
                    embedding: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                });
            }
        }

        entities
    }

    pub async fn auto_link(&self, entities: &[Entity]) -> Vec<Relationship> {
        let mut relationships = Vec::new();

        for i in 0..entities.len() {
            for j in (i + 1)..entities.len().min(i + 3) {
                if entities[i]
                    .name
                    .to_lowercase()
                    .contains(&entities[j].name.to_lowercase())
                    || entities[j]
                        .name
                        .to_lowercase()
                        .contains(&entities[i].name.to_lowercase())
                {
                    relationships.push(Relationship {
                        id: format!("rel_{}", uuid::Uuid::new_v4()),
                        source: entities[i].id.clone(),
                        target: entities[j].id.clone(),
                        relation_type: RelationType::RelatedTo,
                        properties: HashMap::new(),
                        weight: 1.0,
                        created_at: chrono::Utc::now(),
                    });
                }
            }
        }

        relationships
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_get_entity() {
        let graph = KnowledgeGraph::new();

        let entity = Entity {
            id: EntityId("test_1".to_string()),
            name: "Test Company".to_string(),
            entity_type: EntityType::Organization,
            properties: HashMap::new(),
            description: Some("A test organization".to_string()),
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = graph.add_entity(entity.clone()).await;
        assert_eq!(id.0, "test_1");

        let retrieved = graph.get_entity(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Company");
    }

    #[tokio::test]
    async fn test_path_finding() {
        let graph = KnowledgeGraph::new();

        let e1 = Entity {
            id: EntityId("A".to_string()),
            name: "Alice".to_string(),
            entity_type: EntityType::Person,
            properties: HashMap::new(),
            description: None,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let e2 = Entity {
            id: EntityId("B".to_string()),
            name: "Bob".to_string(),
            entity_type: EntityType::Person,
            properties: HashMap::new(),
            description: None,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let e3 = Entity {
            id: EntityId("C".to_string()),
            name: "Charlie".to_string(),
            entity_type: EntityType::Person,
            properties: HashMap::new(),
            description: None,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        graph.add_entity(e1).await;
        graph.add_entity(e2).await;
        graph.add_entity(e3).await;

        graph
            .add_relationship(Relationship {
                id: "r1".to_string(),
                source: EntityId("A".to_string()),
                target: EntityId("B".to_string()),
                relation_type: RelationType::WorksFor,
                properties: HashMap::new(),
                weight: 1.0,
                created_at: chrono::Utc::now(),
            })
            .await;

        graph
            .add_relationship(Relationship {
                id: "r2".to_string(),
                source: EntityId("B".to_string()),
                target: EntityId("C".to_string()),
                relation_type: RelationType::CollaboratesWith,
                properties: HashMap::new(),
                weight: 1.0,
                created_at: chrono::Utc::now(),
            })
            .await;

        let path = graph
            .find_path(&EntityId("A".to_string()), &EntityId("C".to_string()), 10)
            .await;
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3);
    }
}
