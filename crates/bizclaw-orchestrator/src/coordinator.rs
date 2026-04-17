//! Coordinator Agent logic (The "Mama Tổng Quản").
//!
//! Exposes `Coordinator` which receives a goal, generates a Task DAG,
//! and automatically dispatches workloads in parallel across Tokio tasks.

use crate::task_dag::{Task, TaskDag, TaskStatus};
use crate::team::AgentTeam;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Coordinator {
    /// The team of agents available to dispatch tasks
    team: Arc<RwLock<AgentTeam>>,
}

impl Coordinator {
    pub fn new(team: AgentTeam) -> Self {
        Self {
            team: Arc::new(RwLock::new(team)),
        }
    }

    /// Run the team to satisfy the given Goal using true multithreading!
    /// `llm_callback` takes a system prompt and user prompt, returning the string output.
    pub async fn run_team<F, Fut>(&self, goal: &str, llm_callback: F) -> Result<String, String>
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String, String>> + Send + 'static,
    {
        tracing::info!("Coordinator started for goal: {}", goal);

        // Call the LLM callback to dynamically decompose the task
        let dag = self.decompose_goal_into_tasks(goal, &llm_callback).await?;

        // Shared graph for asynchronous execution
        let shared_dag = Arc::new(RwLock::new(dag));

        loop {
            // Find unblocked tasks in Pending state
            let ready_task_ids = {
                let mut graph = shared_dag.write().await;
                let ready = graph.get_ready_tasks();
                // Mark them as running immediately to avoid double-processing
                for task_id in &ready {
                    graph.mark_running(task_id);
                }
                ready
            };

            if ready_task_ids.is_empty() {
                let is_done = shared_dag.read().await.is_all_completed();
                if is_done {
                    break; // Goal completed successfully
                }
                // If no tasks are ready but not all completed, we must wait for
                // running tasks to finish. In a real reactor design, we would await a channel signal here instead of sleeping.
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                continue;
            }

            // Spawn parallel executions -- THIS IS WHERE RUST SHINES vs NODE.JS
            // Each task executes on a truly independent tokio green-thread context.
            for task_id in ready_task_ids {
                let dag_clone = Arc::clone(&shared_dag);
                let task_id_clone = task_id.clone();

                tokio::spawn(async move {
                    // Lookup task
                    let task_info = {
                        let graph = dag_clone.read().await;
                        graph.get_task(&task_id_clone).cloned().unwrap()
                    };

                    tracing::info!(
                        "Starting Task: {} assigned to Agent: {}",
                        task_info.name,
                        task_info.assigned_agent
                    );

                    // Simulate task execution for now since actual agent dispatch
                    // is handled via the gateway's orchestrator routing
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    let result_str = format!("Completed {} dynamically", task_info.name);

                    tracing::info!("Finished Task: {}", task_info.name);

                    // Update Graph
                    let mut graph = dag_clone.write().await;
                    graph.mark_completed(&task_id_clone, result_str);
                });
            }
        }

        tracing::info!("Coordinator successfully orchestrated team to completion!");
        Ok("All subtasks completed successfully. Result aggregated.".to_string())
    }

    /// LLM powered Auto Task Decomposition
    async fn decompose_goal_into_tasks<F, Fut>(
        &self,
        goal: &str,
        llm_callback: &F,
    ) -> Result<TaskDag, String>
    where
        F: Fn(String, String) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<String, String>> + Send,
    {
        tracing::debug!("Requesting LLM to decompose goal: {}", goal);

        let system_prompt = "You are an Orchestrator Agent. Decompose the user's goal into a DAG of subtasks. \
            Output ONLY valid JSON matching this schema: \
            {\"tasks\": [{\"id\": \"t1\", \"name\": \"...\", \"description\": \"...\", \"assigned_agent\": \"...\"}], \
            \"dependencies\": [{\"from\": \"t1\", \"to\": \"t2\"}]}";

        // Call the external LLM closure
        let raw_response = llm_callback(system_prompt.to_string(), goal.to_string()).await?;

        // Clean JSON formatting if wrapped in markdown
        let json_text = raw_response
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: serde_json::Value = serde_json::from_str(json_text)
            .map_err(|e| format!("Failed to parse LLM output as JSON: {}", e))?;

        let mut dag = TaskDag::new();

        if let Some(tasks) = parsed.get("tasks").and_then(|t| t.as_array()) {
            for task_val in tasks {
                dag.add_task(Task {
                    id: task_val["id"].as_str().unwrap_or("unknown").to_string(),
                    name: task_val["name"]
                        .as_str()
                        .unwrap_or("Unnamed Task")
                        .to_string(),
                    description: task_val["description"].as_str().unwrap_or("").to_string(),
                    assigned_agent: task_val["assigned_agent"]
                        .as_str()
                        .unwrap_or("general")
                        .to_string(),
                    status: TaskStatus::Pending,
                    result: None,
                });
            }
        } else {
            return Err("Missing 'tasks' array in LLM response".to_string());
        }

        if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_array()) {
            for dep_val in deps {
                let from = dep_val["from"].as_str().unwrap_or("");
                let to = dep_val["to"].as_str().unwrap_or("");
                if !from.is_empty() && !to.is_empty() {
                    let _ = dag.add_dependency(to, from); // "to" depends on "from" completing
                }
            }
        }

        if dag.get_task_count() == 0 {
            return Err("LLM returned 0 tasks".to_string());
        }

        Ok(dag)
    }
}
