//! Memory Search tool — explicit tool for agents to search conversation memory

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

/// Memory search tool — allows agent to explicitly search past conversations.
/// Note: Agent also auto-retrieves memory during processing, but this tool
/// gives the agent explicit control over when/what to search.
pub struct MemorySearchTool {
    memory: std::sync::Arc<tokio::sync::Mutex<Option<Box<dyn bizclaw_core::traits::memory::MemoryBackend>>>>,
}

impl MemorySearchTool {
    pub fn new(memory: std::sync::Arc<tokio::sync::Mutex<Option<Box<dyn bizclaw_core::traits::memory::MemoryBackend>>>>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str { "memory_search" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "memory_search".into(),
            description: "Search past conversation memory using FTS5 keyword search. Use this to recall previous interactions, facts, preferences, or decisions discussed earlier.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search keywords to find in past conversations"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results to return (default: 5)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let query = args["query"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'query'".into()))?;
        let limit = args["limit"].as_u64().unwrap_or(5) as usize;

        let mem_lock = self.memory.lock().await;
        let memory = match mem_lock.as_ref() {
            Some(m) => m,
            None => {
                return Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: "Memory backend not available.".into(),
                    success: false,
                });
            }
        };

        match memory.search(query, limit).await {
            Ok(results) => {
                if results.is_empty() {
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("No memories found matching '{query}'."),
                        success: true,
                    })
                } else {
                    let mut output = format!("Found {} past conversation(s) matching '{}':\n\n", results.len(), query);
                    for (i, r) in results.iter().enumerate() {
                        let content = if r.entry.content.len() > 500 {
                            format!("{}...", &r.entry.content[..500])
                        } else {
                            r.entry.content.clone()
                        };
                        output.push_str(&format!(
                            "{}. [{}] (score: {:.2})\n{}\n\n",
                            i + 1,
                            r.entry.created_at.format("%Y-%m-%d %H:%M"),
                            r.score,
                            content,
                        ));
                    }
                    Ok(ToolResult {
                        tool_call_id: String::new(),
                        output,
                        success: true,
                    })
                }
            }
            Err(e) => {
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Memory search error: {e}"),
                    success: false,
                })
            }
        }
    }
}
