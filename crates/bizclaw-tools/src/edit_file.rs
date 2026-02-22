//! Edit file tool — precise text replacements in files

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

pub struct EditFileTool;

impl EditFileTool {
    pub fn new() -> Self { Self }
}

impl Default for EditFileTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str { "edit_file" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "edit_file".into(),
            description: "Make precise text replacements in a file. Finds exact matches of old_text and replaces with new_text. Use read_file first to see current content.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path to edit"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "Exact text to find and replace (must match exactly)"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "Replacement text"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "If true, show what would change without modifying file"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let path = args["path"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'path'".into()))?;
        let old_text = args["old_text"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'old_text'".into()))?;
        let new_text = args["new_text"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'new_text'".into()))?;
        let dry_run = args["dry_run"].as_bool().unwrap_or(false);

        // Read current content
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Failed to read {path}: {e}")))?;

        // Count occurrences
        let count = content.matches(old_text).count();
        if count == 0 {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("No match found for the specified text in {path}. Make sure old_text matches exactly (including whitespace and newlines)."),
                success: false,
            });
        }

        if dry_run {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("DRY RUN: Found {count} occurrence(s) of old_text in {path}. Would replace with new_text."),
                success: true,
            });
        }

        // Replace
        let new_content = content.replace(old_text, new_text);
        tokio::fs::write(path, &new_content).await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Failed to write {path}: {e}")))?;

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!("Replaced {count} occurrence(s) in {path} ({} → {} bytes)", content.len(), new_content.len()),
            success: true,
        })
    }
}
