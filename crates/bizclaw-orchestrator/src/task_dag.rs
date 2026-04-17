//! Directed Acyclic Graph (DAG) for multi-agent task execution.
//!
//! Translates goals into a graph of tasks where dependencies are encoded natively.
//! Only tasks that have all dependencies fulfilled can be scheduled.

use petgraph::algo;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Blocked, // Waiting on dependencies
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub assigned_agent: String,
    pub status: TaskStatus,
    /// Result payload (JSON/String) when completed
    pub result: Option<String>,
}

pub struct TaskDag {
    /// The actual directed graph
    graph: DiGraph<Task, ()>,
    /// Map Task ID to NodeIndex to lookup easily
    node_map: HashMap<String, NodeIndex>,
}

impl TaskDag {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a task to the DAG
    pub fn add_task(&mut self, task: Task) {
        let id = task.id.clone();
        let idx = self.graph.add_node(task);
        self.node_map.insert(id, idx);
    }

    /// Mark that task `dependency_id` MUST finish BEFORE `dependent_id` can start
    pub fn add_dependency(
        &mut self,
        dependent_id: &str,
        dependency_id: &str,
    ) -> Result<(), String> {
        let dep_idx = *self
            .node_map
            .get(dependency_id)
            .ok_or("Dependency not found")?;
        let target_idx = *self
            .node_map
            .get(dependent_id)
            .ok_or("Dependent task not found")?;

        // Edge goes from dependency -> dependent
        self.graph.add_edge(dep_idx, target_idx, ());

        // Detect cycle
        if algo::is_cyclic_directed(&self.graph) {
            self.graph
                .remove_edge(self.graph.find_edge(dep_idx, target_idx).unwrap());
            return Err("Adding this dependency creates a cycle!".to_string());
        }

        Ok(())
    }

    /// Get all tasks that are ready to run (i.e. status is Pending and all predecessors are Completed)
    pub fn get_ready_tasks(&mut self) -> Vec<String> {
        let mut ready = Vec::new();

        for idx in self.graph.node_indices() {
            if self.graph[idx].status == TaskStatus::Pending {
                let mut ready_to_run = true;

                // Check incoming edges (predecessors)
                let mut incoming = self
                    .graph
                    .neighbors_directed(idx, petgraph::Direction::Incoming);
                while let Some(pred_idx) = incoming.next() {
                    if self.graph[pred_idx].status != TaskStatus::Completed {
                        ready_to_run = false;
                        break;
                    }
                }

                if ready_to_run {
                    ready.push(self.graph[idx].id.clone());
                }
            }
        }

        ready
    }

    /// Mark task as running
    pub fn mark_running(&mut self, task_id: &str) {
        if let Some(&idx) = self.node_map.get(task_id) {
            self.graph[idx].status = TaskStatus::Running;
        }
    }

    /// Mark task as completed
    pub fn mark_completed(&mut self, task_id: &str, result: String) {
        if let Some(&idx) = self.node_map.get(task_id) {
            self.graph[idx].status = TaskStatus::Completed;
            self.graph[idx].result = Some(result);
        }
    }

    /// Get task by id
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.node_map.get(task_id).map(|&idx| &self.graph[idx])
    }

    /// Check if entire DAG is completed
    pub fn is_all_completed(&self) -> bool {
        self.graph
            .node_weights()
            .all(|task| task.status == TaskStatus::Completed)
    }

    /// Get total number of tasks in DAG
    pub fn get_task_count(&self) -> usize {
        self.graph.node_count()
    }
}
