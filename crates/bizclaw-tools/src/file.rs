//! File read/write tool — read, write, append files with ls-style directory listing.

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
            description: "Read, write, or list files and directories. The list action shows detailed info (type, size, modified time).".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["read", "write", "append", "list"],
                        "description": "Action: read (file contents), write (create/overwrite), append (add to end), list (directory listing with metadata)"
                    },
                    "path": { "type": "string", "description": "File or directory path" },
                    "content": { "type": "string", "description": "Content for write/append actions" },
                    "start_line": { "type": "integer", "description": "Start line for partial read (1-indexed, optional)" },
                    "end_line": { "type": "integer", "description": "End line for partial read (1-indexed, optional)" }
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
                let content = tokio::fs::read_to_string(path).await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Read failed: {e}")))?;

                // Support partial reads with line ranges
                let start = args["start_line"].as_u64().map(|l| l as usize);
                let end = args["end_line"].as_u64().map(|l| l as usize);

                if let (Some(s), Some(e)) = (start, end) {
                    let lines: Vec<&str> = content.lines().collect();
                    let s = s.saturating_sub(1).min(lines.len());
                    let e = e.min(lines.len());
                    let total = lines.len();
                    let selected: Vec<String> = lines[s..e].iter()
                        .enumerate()
                        .map(|(i, l)| format!("{:>4}: {}", s + i + 1, l))
                        .collect();
                    format!("File: {} ({} total lines, showing {}-{}):\n{}", path, total, s + 1, e, selected.join("\n"))
                } else {
                    // If file is very large, show line count
                    let line_count = content.lines().count();
                    if content.len() > 10000 {
                        format!("File: {} ({} lines, {} bytes):\n{}...\n[truncated at 10000 bytes]",
                            path, line_count, content.len(), &content[..10000])
                    } else {
                        content
                    }
                }
            }
            "write" => {
                let content = args["content"].as_str().unwrap_or("");
                // Create parent directories if needed
                if let Some(parent) = std::path::Path::new(path).parent() {
                    tokio::fs::create_dir_all(parent).await
                        .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Create dir: {e}")))?;
                }
                tokio::fs::write(path, content).await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;
                format!("Written {} bytes to {path}", content.len())
            }
            "append" => {
                let content = args["content"].as_str().unwrap_or("");
                use tokio::io::AsyncWriteExt;
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Open failed: {e}")))?;
                file.write_all(content.as_bytes()).await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Write failed: {e}")))?;
                format!("Appended {} bytes to {path}", content.len())
            }
            "list" => {
                let mut entries_result = tokio::fs::read_dir(path).await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

                let mut items = Vec::new();
                while let Some(entry) = entries_result.next_entry().await
                    .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))? {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let meta = entry.metadata().await.ok();
                    let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let modified = meta.as_ref()
                        .and_then(|m| m.modified().ok())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Utc> = t.into();
                            dt.format("%Y-%m-%d %H:%M").to_string()
                        })
                        .unwrap_or_else(|| "?".into());

                    let kind = if is_dir { "dir " } else { "file" };
                    let size_str = if is_dir {
                        "-".to_string()
                    } else {
                        format_size(size)
                    };

                    items.push(format!("{kind}  {size_str:>8}  {modified}  {name}"));
                }

                items.sort();
                if items.is_empty() {
                    format!("Directory {} is empty", path)
                } else {
                    format!("Directory: {} ({} entries)\n{}", path, items.len(), items.join("\n"))
                }
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

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
