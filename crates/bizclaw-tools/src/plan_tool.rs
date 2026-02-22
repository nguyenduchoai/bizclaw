//! Plan Mode tool — structured task decomposition with dependency graphs
//!
//! Plans progress through: Draft → PendingApproval → Approved → InProgress → Completed
//! Tasks have 10 types: Research, Edit, Create, Delete, Test, Refactor, Documentation, Configuration, Build, Other
//! Each task tracks: status, dependencies, complexity (1-5), and timestamps.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Plan state machine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    PendingApproval,
    Approved,
    InProgress,
    Completed,
    Rejected,
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::PendingApproval => write!(f, "Pending Approval"),
            Self::Approved => write!(f, "Approved"),
            Self::InProgress => write!(f, "In Progress"),
            Self::Completed => write!(f, "Completed"),
            Self::Rejected => write!(f, "Rejected"),
        }
    }
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Failed,
    Blocked,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "⏹ Pending"),
            Self::InProgress => write!(f, "▶ In Progress"),
            Self::Completed => write!(f, "✅ Completed"),
            Self::Skipped => write!(f, "⏭ Skipped"),
            Self::Failed => write!(f, "❌ Failed"),
            Self::Blocked => write!(f, "🚫 Blocked"),
        }
    }
}

/// Task type categories (10 types)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Research,
    Edit,
    Create,
    Delete,
    Test,
    Refactor,
    Documentation,
    Configuration,
    Build,
    Other,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Research => write!(f, "🔍 Research"),
            Self::Edit => write!(f, "✏️ Edit"),
            Self::Create => write!(f, "📝 Create"),
            Self::Delete => write!(f, "🗑️ Delete"),
            Self::Test => write!(f, "🧪 Test"),
            Self::Refactor => write!(f, "🔧 Refactor"),
            Self::Documentation => write!(f, "📖 Documentation"),
            Self::Configuration => write!(f, "⚙️ Configuration"),
            Self::Build => write!(f, "🏗️ Build"),
            Self::Other => write!(f, "📌 Other"),
        }
    }
}

/// A single task within a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: usize,
    pub title: String,
    pub description: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub complexity: u8, // 1-5
    pub dependencies: Vec<usize>, // IDs of tasks this depends on
    pub created_at: String,
    pub completed_at: Option<String>,
    pub result: Option<String>,
}

/// A structured execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: PlanStatus,
    pub tasks: Vec<PlanTask>,
    pub created_at: String,
    pub updated_at: String,
}

impl Plan {
    fn new(title: &str, description: &str) -> Self {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            title: title.to_string(),
            description: description.to_string(),
            status: PlanStatus::Draft,
            tasks: vec![],
            created_at: now.clone(),
            updated_at: now,
        }
    }

    fn add_task(&mut self, title: &str, description: &str, task_type: TaskType, complexity: u8, dependencies: Vec<usize>) {
        let id = self.tasks.len() + 1;
        self.tasks.push(PlanTask {
            id,
            title: title.to_string(),
            description: description.to_string(),
            task_type,
            status: TaskStatus::Pending,
            complexity: complexity.clamp(1, 5),
            dependencies,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            completed_at: None,
            result: None,
        });
        self.updated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    }

    fn total_complexity(&self) -> u32 {
        self.tasks.iter().map(|t| t.complexity as u32).sum()
    }

    fn complexity_label(&self) -> &str {
        let total = self.total_complexity();
        match total {
            0..=5 => "Low",
            6..=12 => "Medium",
            13..=20 => "High",
            _ => "Very High",
        }
    }

    fn progress(&self) -> (usize, usize) {
        let completed = self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        (completed, self.tasks.len())
    }

    fn display(&self) -> String {
        let (done, total) = self.progress();
        let stars = |n: u8| "⭐".repeat(n as usize);

        let mut out = format!(
            "📋 Plan: {}\n{}\nStatus: {} • Tasks: {} • Complexity: {} • Progress: {}/{}\n",
            self.title, self.description, self.status, total, self.complexity_label(), done, total
        );
        out.push_str(&"─".repeat(60));
        out.push('\n');

        for task in &self.tasks {
            let deps = if task.dependencies.is_empty() {
                String::new()
            } else {
                format!(" → depends on #{}", task.dependencies.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", #"))
            };
            out.push_str(&format!(
                "  {}. [{}] {} ({}) {}{}\n",
                task.id, task.status, task.title, task.task_type, stars(task.complexity), deps
            ));
            if !task.description.is_empty() {
                out.push_str(&format!("     {}\n", task.description));
            }
            if let Some(result) = &task.result {
                out.push_str(&format!("     Result: {}\n", result));
            }
        }
        out
    }
}

/// Shared plan store
pub type PlanStore = Arc<Mutex<Vec<Plan>>>;

pub fn new_plan_store() -> PlanStore {
    Arc::new(Mutex::new(Vec::new()))
}

/// Plan Mode tool
pub struct PlanTool {
    store: PlanStore,
}

impl PlanTool {
    pub fn new(store: PlanStore) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for PlanTool {
    fn name(&self) -> &str { "plan" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "plan".into(),
            description: "Create and manage structured execution plans. Plans break complex tasks into reviewable, trackable steps with dependencies and complexity ratings.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["create", "add_task", "finalize", "approve", "reject",
                                 "start_task", "complete_task", "fail_task", "skip_task",
                                 "list", "show", "delete"],
                        "description": "Operation to perform"
                    },
                    "plan_id": {
                        "type": "string",
                        "description": "Plan ID (for operations on existing plans)"
                    },
                    "title": {
                        "type": "string",
                        "description": "Plan or task title"
                    },
                    "description": {
                        "type": "string",
                        "description": "Plan or task description"
                    },
                    "task_type": {
                        "type": "string",
                        "enum": ["research", "edit", "create", "delete", "test",
                                 "refactor", "documentation", "configuration", "build", "other"],
                        "description": "Task type category"
                    },
                    "complexity": {
                        "type": "integer",
                        "description": "Task complexity 1-5 (1=trivial, 5=very complex)"
                    },
                    "dependencies": {
                        "type": "array",
                        "items": { "type": "integer" },
                        "description": "Task IDs this task depends on"
                    },
                    "task_id": {
                        "type": "integer",
                        "description": "Task ID (for task operations)"
                    },
                    "result": {
                        "type": "string",
                        "description": "Result/notes when completing a task"
                    }
                },
                "required": ["operation"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let operation = args["operation"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'operation'".into()))?;

        let mut store = self.store.lock().await;

        match operation {
            "create" => {
                let title = args["title"].as_str().unwrap_or("Untitled Plan");
                let description = args["description"].as_str().unwrap_or("");
                let plan = Plan::new(title, description);
                let id = plan.id.clone();
                store.push(plan);
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("✅ Plan created (ID: {}). Use add_task to add tasks, then finalize.", id),
                    success: true,
                })
            }

            "add_task" => {
                let plan = find_plan_mut(&mut store, &args)?;
                if plan.status != PlanStatus::Draft {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: "Cannot add tasks — plan is not in Draft status. Create a new plan.".into(),
                        success: false,
                    });
                }
                let title = args["title"].as_str().unwrap_or("Untitled Task");
                let description = args["description"].as_str().unwrap_or("");
                let task_type = match args["task_type"].as_str().unwrap_or("other") {
                    "research" => TaskType::Research,
                    "edit" => TaskType::Edit,
                    "create" => TaskType::Create,
                    "delete" => TaskType::Delete,
                    "test" => TaskType::Test,
                    "refactor" => TaskType::Refactor,
                    "documentation" => TaskType::Documentation,
                    "configuration" => TaskType::Configuration,
                    "build" => TaskType::Build,
                    _ => TaskType::Other,
                };
                let complexity = args["complexity"].as_u64().unwrap_or(2) as u8;
                let dependencies: Vec<usize> = args["dependencies"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
                    .unwrap_or_default();

                plan.add_task(title, description, task_type, complexity, dependencies);
                let task_id = plan.tasks.len();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("✅ Task #{} added: {}", task_id, title),
                    success: true,
                })
            }

            "finalize" => {
                let plan = find_plan_mut(&mut store, &args)?;
                if plan.tasks.is_empty() {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: "Cannot finalize — plan has no tasks.".into(),
                        success: false,
                    });
                }
                plan.status = PlanStatus::PendingApproval;
                plan.updated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let display = plan.display();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("✅ Plan finalized and ready for review!\n\n{}", display),
                    success: true,
                })
            }

            "approve" => {
                let plan = find_plan_mut(&mut store, &args)?;
                if plan.status != PlanStatus::PendingApproval {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Cannot approve — plan is in '{}' status, not PendingApproval.", plan.status),
                        success: false,
                    });
                }
                plan.status = PlanStatus::Approved;
                plan.updated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("✅ Plan '{}' approved! Start executing tasks with start_task.", plan.title),
                    success: true,
                })
            }

            "reject" => {
                let plan = find_plan_mut(&mut store, &args)?;
                plan.status = PlanStatus::Rejected;
                plan.updated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("❌ Plan '{}' rejected.", plan.title),
                    success: true,
                })
            }

            "start_task" => {
                let plan = find_plan_mut(&mut store, &args)?;
                if plan.status != PlanStatus::Approved && plan.status != PlanStatus::InProgress {
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: "Cannot start task — plan must be Approved first.".into(),
                        success: false,
                    });
                }
                plan.status = PlanStatus::InProgress;
                let task_id = args["task_id"].as_u64().unwrap_or(0) as usize;

                // First: check dependencies (immutable pass)
                let task_idx = plan.tasks.iter().position(|t| t.id == task_id);
                if let Some(idx) = task_idx {
                    let deps = plan.tasks[idx].dependencies.clone();
                    for dep_id in &deps {
                        if let Some(dep) = plan.tasks.iter().find(|t| t.id == *dep_id) {
                            if dep.status != TaskStatus::Completed {
                                return Ok(ToolResult {
                                    tool_call_id: String::new(),
                                    output: format!("🚫 Cannot start task #{} — dependency #{} is not completed ({})", task_id, dep_id, dep.status),
                                    success: false,
                                });
                            }
                        }
                    }
                    // Now mutate
                    plan.tasks[idx].status = TaskStatus::InProgress;
                    let title = plan.tasks[idx].title.clone();
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("▶ Task #{} started: {}", task_id, title),
                        success: true,
                    })
                } else {
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Task #{} not found", task_id),
                        success: false,
                    })
                }
            }

            "complete_task" => {
                let plan = find_plan_mut(&mut store, &args)?;
                let task_id = args["task_id"].as_u64().unwrap_or(0) as usize;
                let result_text = args["result"].as_str().map(|s| s.to_string());
                if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = TaskStatus::Completed;
                    task.completed_at = Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
                    task.result = result_text;

                    // Check if all tasks completed
                    let all_done = plan.tasks.iter().all(|t| {
                        t.status == TaskStatus::Completed || t.status == TaskStatus::Skipped
                    });
                    if all_done {
                        plan.status = PlanStatus::Completed;
                    }
                    plan.updated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

                    let msg = if all_done {
                        format!("✅ Task #{} completed! 🎉 All tasks done — plan completed!", task_id)
                    } else {
                        let (done, total) = plan.progress();
                        format!("✅ Task #{} completed! Progress: {}/{}", task_id, done, total)
                    };
                    Ok(ToolResult { tool_call_id: String::new(), output: msg, success: true })
                } else {
                    Ok(ToolResult { tool_call_id: String::new(), output: format!("Task #{} not found", task_id), success: false })
                }
            }

            "fail_task" => {
                let plan = find_plan_mut(&mut store, &args)?;
                let task_id = args["task_id"].as_u64().unwrap_or(0) as usize;
                let result_text = args["result"].as_str().map(|s| s.to_string());
                if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = TaskStatus::Failed;
                    task.result = result_text;
                    Ok(ToolResult { tool_call_id: String::new(), output: format!("❌ Task #{} failed: {}", task_id, task.title), success: true })
                } else {
                    Ok(ToolResult { tool_call_id: String::new(), output: format!("Task #{} not found", task_id), success: false })
                }
            }

            "skip_task" => {
                let plan = find_plan_mut(&mut store, &args)?;
                let task_id = args["task_id"].as_u64().unwrap_or(0) as usize;
                if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = TaskStatus::Skipped;
                    Ok(ToolResult { tool_call_id: String::new(), output: format!("⏭ Task #{} skipped: {}", task_id, task.title), success: true })
                } else {
                    Ok(ToolResult { tool_call_id: String::new(), output: format!("Task #{} not found", task_id), success: false })
                }
            }

            "list" => {
                if store.is_empty() {
                    return Ok(ToolResult { tool_call_id: String::new(), output: "No plans exist yet.".into(), success: true });
                }
                let mut out = format!("📋 {} plan(s):\n\n", store.len());
                for plan in store.iter() {
                    let (done, total) = plan.progress();
                    out.push_str(&format!("  [{}] {} — {} ({}/{} tasks)\n", plan.id, plan.title, plan.status, done, total));
                }
                Ok(ToolResult { tool_call_id: String::new(), output: out, success: true })
            }

            "show" => {
                let plan = find_plan(&store, &args)?;
                Ok(ToolResult { tool_call_id: String::new(), output: plan.display(), success: true })
            }

            "delete" => {
                let plan_id = get_plan_id(&store, &args)?;
                store.retain(|p| p.id != plan_id);
                Ok(ToolResult { tool_call_id: String::new(), output: format!("🗑️ Plan {} deleted.", plan_id), success: true })
            }

            _ => Err(bizclaw_core::error::BizClawError::Tool(format!("Unknown operation: {operation}"))),
        }
    }
}

fn get_plan_id(store: &[Plan], args: &serde_json::Value) -> Result<String> {
    if let Some(id) = args["plan_id"].as_str() {
        Ok(id.to_string())
    } else if store.len() == 1 {
        Ok(store[0].id.clone())
    } else if let Some(last) = store.last() {
        Ok(last.id.clone())
    } else {
        Err(bizclaw_core::error::BizClawError::Tool("No plan_id specified and no plans exist".into()))
    }
}

fn find_plan<'a>(store: &'a [Plan], args: &serde_json::Value) -> Result<&'a Plan> {
    let id = get_plan_id(store, args)?;
    store.iter().find(|p| p.id == id)
        .ok_or_else(|| bizclaw_core::error::BizClawError::Tool(format!("Plan '{}' not found", id)))
}

fn find_plan_mut<'a>(store: &'a mut Vec<Plan>, args: &serde_json::Value) -> Result<&'a mut Plan> {
    let id = get_plan_id(store, args)?;
    store.iter_mut().find(|p| p.id == id)
        .ok_or_else(|| bizclaw_core::error::BizClawError::Tool(format!("Plan '{}' not found", id)))
}
