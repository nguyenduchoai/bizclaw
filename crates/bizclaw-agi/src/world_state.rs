//! World State Tracking - Snapshot per iteration
//! 
//! Giống AGNT world-state tracking.
//! Track tất cả state changes qua mỗi iteration để có thể revert.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// A snapshot of world state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub id: String,
    pub iteration: u32,
    pub timestamp: DateTime<Utc>,
    pub state: WorldStateData,
    pub diff_from_previous: Vec<StateChange>,
}

/// World state data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldStateData {
    pub entities: HashMap<String, Entity>,
    pub variables: HashMap<String, serde_json::Value>,
    pub resources: HashMap<String, Resource>,
    pub relationships: Vec<Relationship>,
}

/// An entity in the world
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A resource (file, API, database, etc)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    pub resource_type: ResourceType,
    pub path: Option<String>,
    pub status: ResourceStatus,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    File,
    Database,
    Api,
    Service,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceStatus {
    Available,
    InUse,
    Locked,
    Unavailable,
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub relation_type: String,
}

/// A change to the world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub change_type: ChangeType,
    pub entity_id: Option<String>,
    pub field: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
    Accessed,
}

/// World State Tracker
#[derive(Debug, Clone)]
pub struct WorldState {
    pub data: WorldStateData,
    pub snapshots: Vec<StateSnapshot>,
    pub history: Vec<StateChange>,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            data: WorldStateData::default(),
            snapshots: Vec::new(),
            history: Vec::new(),
        }
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let id = Uuid::new_v4().to_string();
        let iteration = self.snapshots.len() as u32;
        
        let diff_from_previous = if let Some(prev) = self.snapshots.last() {
            self.compute_diff(&prev.state, &self.data)
        } else {
            Vec::new()
        };

        

        StateSnapshot {
            id,
            iteration,
            timestamp: Utc::now(),
            state: self.data.clone(),
            diff_from_previous,
        }
    }

    pub fn save_snapshot(&mut self) -> String {
        let snapshot = self.snapshot();
        let id = snapshot.id.clone();
        self.snapshots.push(snapshot);
        id
    }

    pub fn revert_to(&mut self, snapshot_id: &str) -> Result<(), String> {
        let idx = self.snapshots
            .iter()
            .position(|s| s.id == snapshot_id)
            .ok_or_else(|| format!("Snapshot not found: {}", snapshot_id))?;

        let snapshot = self.snapshots[idx].clone();
        self.data = snapshot.state.clone();
        self.snapshots.truncate(idx + 1);
        
        Ok(())
    }

    pub fn revert_to_iteration(&mut self, iteration: u32) -> Result<(), String> {
        let snapshot_id = {
            let snapshot = self.snapshots
                .iter()
                .find(|s| s.iteration == iteration)
                .ok_or_else(|| format!("Snapshot for iteration {} not found", iteration))?;
            snapshot.id.clone()
        };
        
        self.revert_to(&snapshot_id)
    }

    pub fn update_entity(&mut self, entity: Entity) {
        let change = if self.data.entities.contains_key(&entity.id) {
            StateChange {
                change_type: ChangeType::Updated,
                entity_id: Some(entity.id.clone()),
                field: None,
                old_value: self.data.entities.get(&entity.id)
                    .map(|e| serde_json::to_value(e).unwrap_or_default()),
                new_value: Some(serde_json::to_value(&entity).unwrap_or_default()),
                timestamp: Utc::now(),
            }
        } else {
            StateChange {
                change_type: ChangeType::Created,
                entity_id: Some(entity.id.clone()),
                field: None,
                old_value: None,
                new_value: Some(serde_json::to_value(&entity).unwrap_or_default()),
                timestamp: Utc::now(),
            }
        };

        self.history.push(change);
        let id = entity.id.clone();
        self.data.entities.insert(id, entity);
    }

    pub fn delete_entity(&mut self, entity_id: &str) -> Option<Entity> {
        if let Some(entity) = self.data.entities.remove(entity_id) {
            let change = StateChange {
                change_type: ChangeType::Deleted,
                entity_id: Some(entity_id.to_string()),
                field: None,
                old_value: Some(serde_json::to_value(&entity).unwrap_or_default()),
                new_value: None,
                timestamp: Utc::now(),
            };
            self.history.push(change);
            Some(entity)
        } else {
            None
        }
    }

    pub fn set_variable(&mut self, key: &str, value: serde_json::Value) {
        let old_value = self.data.variables.get(key).cloned();
        
        let change = StateChange {
            change_type: ChangeType::Updated,
            entity_id: None,
            field: Some(key.to_string()),
            old_value,
            new_value: Some(value.clone()),
            timestamp: Utc::now(),
        };

        self.history.push(change);
        self.data.variables.insert(key.to_string(), value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.variables.get(key)
    }

    pub fn add_resource(&mut self, resource: Resource) {
        self.data.resources.insert(resource.id.clone(), resource);
    }

    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.data.relationships.push(relationship);
    }

    pub fn get_snapshots(&self) -> &[StateSnapshot] {
        &self.snapshots
    }

    pub fn get_history(&self) -> &[StateChange] {
        &self.history
    }

    pub fn data_clone(&self) -> WorldStateData {
        self.data.clone()
    }

    pub fn update_from_execution(&mut self, step: &ExecutionStep) {
        self.set_variable(&format!("last_step_{}", step.step), serde_json::json!(step.result));
        if step.success {
            self.set_variable("last_success", serde_json::json!(true));
        } else {
            self.set_variable("last_error", serde_json::json!(step.result));
        }
    }

    fn compute_diff(&self, old: &WorldStateData, new: &WorldStateData) -> Vec<StateChange> {
        let mut diff = Vec::new();

        // Check entities
        for (id, new_entity) in &new.entities {
            if let Some(old_entity) = old.entities.get(id) {
                if old_entity != new_entity {
                    diff.push(StateChange {
                        change_type: ChangeType::Updated,
                        entity_id: Some(id.clone()),
                        field: None,
                        old_value: Some(serde_json::to_value(old_entity).unwrap_or_default()),
                        new_value: Some(serde_json::to_value(new_entity).unwrap_or_default()),
                        timestamp: Utc::now(),
                    });
                }
            } else {
                diff.push(StateChange {
                    change_type: ChangeType::Created,
                    entity_id: Some(id.clone()),
                    field: None,
                    old_value: None,
                    new_value: Some(serde_json::to_value(new_entity).unwrap_or_default()),
                    timestamp: Utc::now(),
                });
            }
        }

        // Check deleted entities
        for id in old.entities.keys() {
            if !new.entities.contains_key(id) {
                diff.push(StateChange {
                    change_type: ChangeType::Deleted,
                    entity_id: Some(id.clone()),
                    field: None,
                    old_value: Some(serde_json::to_value(&old.entities[id]).unwrap_or_default()),
                    new_value: None,
                    timestamp: Utc::now(),
                });
            }
        }

        diff
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

/// A step in execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step: String,
    pub tool: Option<String>,
    pub result: String,
    pub success: bool,
    pub duration_ms: u64,
}
