//! File read/write tool.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

pub struct FileTool;

impl FileTool {
    pub fn new() -> Self { Self }
}

impl Default for FileTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &str { "file" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file".into(),
            description: "Read or write files.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["read", "write", "list"] },
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["action", "path"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let action = args["action"].as_str().unwrap_or("read");
        let path = args["path"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'path'".into()))?;

        let result = match action {
            "read" => {
                tokio::fs::read_to_string(path).await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?
            }
            "write" => {
                let content = args["content"].as_str().unwrap_or("");
                tokio::fs::write(path, content).await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;
                format!("Written {} bytes to {path}", content.len())
            }
            "list" => {
                let mut entries = tokio::fs::read_dir(path).await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;
                let mut result = Vec::new();
                while let Some(entry) = entries.next_entry().await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))? {
                    result.push(entry.file_name().to_string_lossy().to_string());
                }
                result.join("\n")
            }
            _ => return Err(bizclaw_core::error::BizClawError::Tool(format!("Unknown action: {action}"))),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: result,
            success: true,
        })
    }
}
