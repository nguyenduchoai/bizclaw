use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use kreuzberg::{ExtractionConfig, extract_file};
use std::path::Path;

pub struct DocumentReaderTool;

impl DocumentReaderTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocumentReaderTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DocumentReaderTool {
    fn name(&self) -> &str {
        "document_reader"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "document_reader".into(),
            description: "Extracts clean text from documents of any format (PDF, DOCX, XLSX, PPTX, TXT, CSV, HTML, images with OCR, and more). Useful for analyzing contracts, reports, attachments, and files securely without uploading to the cloud.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["read_file"],
                        "description": "Action to perform."
                    },
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the document file on disk."
                    }
                },
                "required": ["action", "path"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");

        if action != "read_file" {
            return Err(bizclaw_core::error::BizClawError::Tool(
                "Invalid action for document_reader. Use 'read_file'".into(),
            ));
        }

        if path_str.is_empty() {
            return Err(bizclaw_core::error::BizClawError::Tool(
                "Missing 'path' argument".into(),
            ));
        }

        let path = Path::new(path_str);
        if !path.exists() {
            return Err(bizclaw_core::error::BizClawError::Tool(format!(
                "File not found: {}",
                path.display()
            )));
        }

        let result = extract_file(path_str, None, &ExtractionConfig::default())
            .await
            .map_err(|e| {
                bizclaw_core::error::BizClawError::Tool(format!("Failed to extract document: {e}"))
            })?;

        let mut content = result.content;
        let char_limit = 100_000;
        if content.len() > char_limit {
            content.truncate(char_limit);
            content.push_str("\n\n[... TEXT TRUNCATED DUE TO LENGTH LIMIT ...]");
        }

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: format!("Extracted content from {}:\n\n{}", path.display(), content),
            success: true,
        })
    }
}
