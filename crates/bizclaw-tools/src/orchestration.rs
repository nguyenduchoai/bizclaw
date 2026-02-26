//! Orchestration tools — agents can call these to interact with the multi-agent system.
//!
//! Tools:
//! - `delegate` — delegate a task to another agent
//! - `handoff` — transfer conversation control
//! - `team_tasks` — view/claim tasks on the shared task board
//! - `team_message` — send/read team messages
//! - `list_agents` — discover available agents

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use bizclaw_db::store::DataStore;

/// Shared orchestration state for tools.
pub type SharedOrchState = Arc<Mutex<OrchToolState>>;

/// State that orchestration tools need access to.
pub struct OrchToolState {
    /// Current agent's name.
    pub agent_name: String,
    /// Available agent names + descriptions.
    pub agents: Vec<(String, String, String)>, // (name, role, description)
    /// Data store.
    pub store: Option<Arc<dyn DataStore>>,
    /// Pending delegation results (from delegate tool calls).
    pub pending_delegations: Vec<PendingDelegation>,
}

/// A pending delegation that the orchestrator needs to execute.
pub struct PendingDelegation {
    pub to_agent: String,
    pub task: String,
    pub mode: String, // "sync" or "async"
}

// ── Delegate Tool ──────────────────────────────────────────

/// Tool for agents to delegate tasks to other agents.
pub struct DelegateTool {
    state: SharedOrchState,
}

impl DelegateTool {
    pub fn new(state: SharedOrchState) -> Self {
        Self { state }
    }
}

#[derive(Deserialize)]
struct DelegateArgs {
    to_agent: String,
    task: String,
    #[serde(default = "default_mode")]
    mode: String,
}

fn default_mode() -> String {
    "sync".to_string()
}

#[async_trait]
impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "delegate"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "delegate".to_string(),
            description: "Delegate a task to another agent. Use when the task is outside your expertise.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "to_agent": {
                        "type": "string",
                        "description": "Name of the agent to delegate to"
                    },
                    "task": {
                        "type": "string",
                        "description": "The task to delegate (clear, actionable instruction)"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["sync", "async"],
                        "default": "sync",
                        "description": "sync = wait for result, async = fire and forget"
                    }
                },
                "required": ["to_agent", "task"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: DelegateArgs = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid args: {e}")))?;

        let mut state = self.state.lock().await;

        // Check agent exists
        let agent_exists = state.agents.iter().any(|(name, _, _)| name == &args.to_agent);
        if !agent_exists {
            let available: Vec<&str> = state.agents.iter().map(|(n, _, _)| n.as_str()).collect();
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "Agent '{}' not found. Available agents: {}",
                    args.to_agent,
                    available.join(", ")
                ),
                success: false,
            });
        }

        // Queue the delegation for the orchestrator to execute
        state.pending_delegations.push(PendingDelegation {
            to_agent: args.to_agent.clone(),
            task: args.task.clone(),
            mode: args.mode.clone(),
        });

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!(
                "Delegation queued: task sent to agent '{}' (mode: {}). The orchestrator will process this.",
                args.to_agent, args.mode
            ),
            success: true,
        })
    }
}

// ── Handoff Tool ───────────────────────────────────────────

/// Tool for agents to transfer conversation control.
pub struct HandoffTool {
    state: SharedOrchState,
}

impl HandoffTool {
    pub fn new(state: SharedOrchState) -> Self {
        Self { state }
    }
}

#[derive(Deserialize)]
struct HandoffArgs {
    to_agent: String,
    reason: Option<String>,
    session_id: Option<String>,
}

#[async_trait]
impl Tool for HandoffTool {
    fn name(&self) -> &str {
        "handoff"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "handoff".to_string(),
            description: "Transfer conversation control to another agent. Use when the user's needs are better served by a different agent.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "to_agent": {
                        "type": "string",
                        "description": "Name of the agent to hand off to"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Why you're transferring control"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session to transfer (uses current if omitted)"
                    }
                },
                "required": ["to_agent"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: HandoffArgs = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid args: {e}")))?;

        let state = self.state.lock().await;

        let from = state.agent_name.clone();
        let session_id = args
            .session_id
            .unwrap_or_else(|| format!("session-{}", from));

        if let Some(store) = &state.store {
            let handoff = bizclaw_core::types::Handoff::new(
                &from,
                &args.to_agent,
                &session_id,
                args.reason.as_deref(),
            );
            let _ = store.create_handoff(&handoff).await;
            Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "Handoff complete: {} → {} (session: {}, reason: {:?})",
                    from, args.to_agent, session_id, args.reason
                ),
                success: true,
            })
        } else {
            Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "Handoff requested: {} → {} (no data store — handoff not persisted)",
                    from, args.to_agent
                ),
                success: true,
            })
        }
    }
}

// ── List Agents Tool ───────────────────────────────────────

/// Tool for agents to discover other available agents.
pub struct ListAgentsTool {
    state: SharedOrchState,
}

impl ListAgentsTool {
    pub fn new(state: SharedOrchState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for ListAgentsTool {
    fn name(&self) -> &str {
        "list_agents"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_agents".to_string(),
            description: "List all available agents in the system with their roles and descriptions.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn execute(&self, _arguments: &str) -> Result<ToolResult> {
        let state = self.state.lock().await;
        let agents_info: Vec<String> = state
            .agents
            .iter()
            .map(|(name, role, desc)| {
                format!("- **{}** ({}): {}", name, role, desc)
            })
            .collect();
        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!("Available Agents:\n{}", agents_info.join("\n")),
            success: true,
        })
    }
}

// ── Team Tasks Tool ────────────────────────────────────────

/// Tool for agents to interact with the team task board.
pub struct TeamTasksTool {
    state: SharedOrchState,
}

impl TeamTasksTool {
    pub fn new(state: SharedOrchState) -> Self {
        Self { state }
    }
}

#[derive(Deserialize)]
struct TeamTasksArgs {
    action: String, // "list", "claim", "complete"
    team_id: Option<String>,
    task_id: Option<String>,
    result: Option<String>,
}

#[async_trait]
impl Tool for TeamTasksTool {
    fn name(&self) -> &str {
        "team_tasks"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "team_tasks".to_string(),
            description: "Interact with the team task board: list, claim, or complete tasks.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "claim", "complete"],
                        "description": "Action to perform"
                    },
                    "team_id": {
                        "type": "string",
                        "description": "Team ID (required for 'list')"
                    },
                    "task_id": {
                        "type": "string",
                        "description": "Task ID (required for 'claim' and 'complete')"
                    },
                    "result": {
                        "type": "string",
                        "description": "Task result (required for 'complete')"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: TeamTasksArgs = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid args: {e}")))?;

        let state = self.state.lock().await;
        let store = match &state.store {
            Some(s) => s,
            None => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: "Team tasks not available (no data store configured)".to_string(),
                    success: false,
                });
            }
        };

        match args.action.as_str() {
            "list" => {
                if let Some(team_id) = &args.team_id {
                    let tasks = store.list_tasks(team_id).await.map_err(|e| {
                        bizclaw_core::error::BizClawError::Tool(e.to_string())
                    })?;
                    let list: Vec<String> = tasks
                        .iter()
                        .map(|t| {
                            format!(
                                "- [{}] {} ({:?}) assigned_to: {:?}",
                                t.id, t.title, t.status, t.assigned_to
                            )
                        })
                        .collect();
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: if list.is_empty() {
                            "No tasks on the board.".to_string()
                        } else {
                            format!("Team Tasks:\n{}", list.join("\n"))
                        },
                        success: true,
                    })
                } else {
                    // List tasks assigned to this agent
                    let tasks = store.list_agent_tasks(&state.agent_name).await.map_err(|e| {
                        bizclaw_core::error::BizClawError::Tool(e.to_string())
                    })?;
                    let list: Vec<String> = tasks
                        .iter()
                        .map(|t| {
                            format!("- [{}] {} ({:?})", t.id, t.title, t.status)
                        })
                        .collect();
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: if list.is_empty() {
                            "No tasks assigned to you.".to_string()
                        } else {
                            format!("Your Tasks:\n{}", list.join("\n"))
                        },
                        success: true,
                    })
                }
            }
            "claim" => {
                let task_id = args.task_id.ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("task_id required for 'claim'".into())
                })?;
                store
                    .update_task(
                        &task_id,
                        bizclaw_core::types::TaskStatus::InProgress,
                        Some(&state.agent_name),
                        None,
                    )
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Task '{}' claimed by '{}'", task_id, state.agent_name),
                    success: true,
                })
            }
            "complete" => {
                let task_id = args.task_id.ok_or_else(|| {
                    bizclaw_core::error::BizClawError::Tool("task_id required for 'complete'".into())
                })?;
                let result = args.result.unwrap_or_else(|| "Completed.".to_string());
                store
                    .update_task(
                        &task_id,
                        bizclaw_core::types::TaskStatus::Completed,
                        None,
                        Some(&result),
                    )
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Task '{}' completed.", task_id),
                    success: true,
                })
            }
            _ => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("Unknown action: '{}'. Use: list, claim, complete", args.action),
                success: false,
            }),
        }
    }
}

// ── Team Message Tool ──────────────────────────────────────

/// Tool for agents to send/read team messages.
pub struct TeamMessageTool {
    state: SharedOrchState,
}

impl TeamMessageTool {
    pub fn new(state: SharedOrchState) -> Self {
        Self { state }
    }
}

#[derive(Deserialize)]
struct TeamMessageArgs {
    action: String, // "send", "read"
    team_id: String,
    to_agent: Option<String>,
    content: Option<String>,
}

#[async_trait]
impl Tool for TeamMessageTool {
    fn name(&self) -> &str {
        "team_message"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "team_message".to_string(),
            description: "Send or read messages in the team mailbox.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["send", "read"],
                        "description": "send = send a message, read = read unread messages"
                    },
                    "team_id": {
                        "type": "string",
                        "description": "Team ID"
                    },
                    "to_agent": {
                        "type": "string",
                        "description": "Target agent (omit for broadcast)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Message content (required for 'send')"
                    }
                },
                "required": ["action", "team_id"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: TeamMessageArgs = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid args: {e}")))?;

        let state = self.state.lock().await;
        let store = match &state.store {
            Some(s) => s,
            None => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: "Team messages not available (no data store configured)".to_string(),
                    success: false,
                });
            }
        };

        match args.action.as_str() {
            "send" => {
                let content = args.content.unwrap_or_else(|| "(empty)".to_string());
                let msg = if let Some(to) = &args.to_agent {
                    bizclaw_core::types::TeamMessage::direct(
                        &args.team_id,
                        &state.agent_name,
                        to,
                        &content,
                    )
                } else {
                    bizclaw_core::types::TeamMessage::broadcast(
                        &args.team_id,
                        &state.agent_name,
                        &content,
                    )
                };
                store.send_team_message(&msg).await.map_err(|e| {
                    bizclaw_core::error::BizClawError::Tool(e.to_string())
                })?;
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: "Message sent.".to_string(),
                    success: true,
                })
            }
            "read" => {
                let messages = store
                    .unread_messages(&args.team_id, &state.agent_name)
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;
                if messages.is_empty() {
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: "No unread messages.".to_string(),
                        success: true,
                    })
                } else {
                    let ids: Vec<String> = messages.iter().map(|m| m.id.clone()).collect();
                    let list: Vec<String> = messages
                        .iter()
                        .map(|m| format!("- From {}: {}", m.from_agent, m.content))
                        .collect();
                    let _ = store.mark_read(&ids).await;
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Unread Messages:\n{}", list.join("\n")),
                        success: true,
                    })
                }
            }
            _ => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("Unknown action: '{}'. Use: send, read", args.action),
                success: false,
            }),
        }
    }
}
