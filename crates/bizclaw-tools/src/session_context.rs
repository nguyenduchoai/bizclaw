//! Session Context tool — access session information

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Session info container — shared between agent and tools.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub created_at: String,
    pub provider: String,
    pub model: String,
    pub message_count: usize,
    pub estimated_tokens: usize,
    pub context_utilization: f32,
    pub max_context: usize,
    pub tools_available: Vec<String>,
    pub workspace_path: String,
    pub brain_enabled: bool,
    pub memory_enabled: bool,
    pub knowledge_docs: usize,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            session_id: "default".into(),
            created_at: chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
            provider: "unknown".into(),
            model: "unknown".into(),
            message_count: 0,
            estimated_tokens: 0,
            context_utilization: 0.0,
            max_context: 128000,
            tools_available: vec![],
            workspace_path: "~/.bizclaw".into(),
            brain_enabled: false,
            memory_enabled: true,
            knowledge_docs: 0,
        }
    }
}

pub type SharedSessionInfo = Arc<Mutex<SessionInfo>>;

pub fn new_session_info() -> SharedSessionInfo {
    Arc::new(Mutex::new(SessionInfo::default()))
}

/// Session context tool — gives the agent awareness of its own session.
pub struct SessionContextTool {
    info: SharedSessionInfo,
}

impl SessionContextTool {
    pub fn new(info: SharedSessionInfo) -> Self {
        Self { info }
    }
}

#[async_trait]
impl Tool for SessionContextTool {
    fn name(&self) -> &str {
        "session_context"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "session_context".into(),
            description: "Get information about the current session: provider, model, token usage, available tools, workspace path, and more. Use this to understand your current capabilities and context.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "field": {
                        "type": "string",
                        "enum": ["all", "session_id", "provider", "model", "tokens",
                                 "context", "tools", "workspace", "uptime"],
                        "description": "Specific field to query (default: all)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value =
            serde_json::from_str(arguments).unwrap_or_else(|_| serde_json::json!({}));

        let field = args["field"].as_str().unwrap_or("all");
        let info = self.info.lock().await;

        let output = match field {
            "session_id" => format!("Session ID: {}", info.session_id),
            "provider" => format!("Provider: {} (model: {})", info.provider, info.model),
            "model" => format!("Model: {}", info.model),
            "tokens" => format!(
                "Estimated tokens: {} / {} ({:.1}%)",
                info.estimated_tokens, info.max_context, info.context_utilization
            ),
            "context" => format!(
                "Context:\n  Messages: {}\n  Tokens: ~{}\n  Utilization: {:.1}%\n  Max context: {}",
                info.message_count,
                info.estimated_tokens,
                info.context_utilization,
                info.max_context
            ),
            "tools" => {
                if info.tools_available.is_empty() {
                    "Available tools: (loading...)".into()
                } else {
                    format!(
                        "Available tools ({}):\n  {}",
                        info.tools_available.len(),
                        info.tools_available.join(", ")
                    )
                }
            }
            "workspace" => format!(
                "Workspace: {}\nBrain: {}\nMemory: {}\nKnowledge docs: {}",
                info.workspace_path,
                if info.brain_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                if info.memory_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                info.knowledge_docs
            ),
            _ => {
                format!(
                    "═══ Session Context ═══\n\
                     Session ID: {}\n\
                     Created: {}\n\
                     Provider: {} ({})\n\
                     \n\
                     Context:\n\
                       Messages: {}\n\
                       Tokens: ~{} / {} ({:.1}%)\n\
                     \n\
                     Capabilities:\n\
                       Tools: {} available\n\
                       Brain: {}\n\
                       Memory: {} (FTS5)\n\
                       Knowledge: {} doc(s)\n\
                     \n\
                     Workspace: {}",
                    info.session_id,
                    info.created_at,
                    info.provider,
                    info.model,
                    info.message_count,
                    info.estimated_tokens,
                    info.max_context,
                    info.context_utilization,
                    info.tools_available.len(),
                    if info.brain_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    if info.memory_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    info.knowledge_docs,
                    info.workspace_path,
                )
            }
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}
