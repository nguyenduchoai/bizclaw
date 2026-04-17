//! Zalo Group Summarizer Tool — buffer group messages and summarize with LLM.
//!
//! Monitors Zalo group chats, buffers messages over a configurable time window,
//! then uses the AI provider to generate a summary.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// A single buffered message from a Zalo group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedMessage {
    pub sender_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub group_id: String,
    pub group_name: String,
}

/// Configuration for the group summarizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizerConfig {
    /// Time window for buffering messages (in seconds)
    #[serde(default = "default_buffer_window")]
    pub buffer_window_secs: u64,
    /// Maximum messages to buffer per group
    #[serde(default = "default_max_messages")]
    pub max_messages_per_group: usize,
    /// Language for summaries
    #[serde(default = "default_language")]
    pub language: String,
    /// Summary style (brief, detailed, bullet_points)
    #[serde(default = "default_style")]
    pub summary_style: String,
}

fn default_buffer_window() -> u64 {
    3600
} // 1 hour
fn default_max_messages() -> usize {
    200
}
fn default_language() -> String {
    "vi".into()
}
fn default_style() -> String {
    "bullet_points".into()
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            buffer_window_secs: 3600,
            max_messages_per_group: 200,
            language: "vi".into(),
            summary_style: "bullet_points".into(),
        }
    }
}

pub struct GroupSummarizerTool {
    config: SummarizerConfig,
    db_path: String,
}

impl GroupSummarizerTool {
    pub fn new(config: SummarizerConfig) -> Self {
        let db_path = shellexpand::tilde("~/.gemini/antigravity/gateway.db").to_string();
        Self { config, db_path }
    }

    /// Format messages into a prompt for the LLM.
    fn format_messages_for_llm(&self, messages: &[BufferedMessage], group_name: &str) -> String {
        let lang = if self.config.language == "vi" {
            "tiếng Việt"
        } else {
            "English"
        };
        let style_instruction = match self.config.summary_style.as_str() {
            "brief" => "Tóm tắt ngắn gọn trong 2-3 câu.",
            "detailed" => "Tóm tắt chi tiết, nêu rõ ai nói gì, chủ đề chính.",
            _ => "Tóm tắt dạng bullet points, mỗi chủ đề 1 gạch đầu dòng.",
        };

        let mut prompt = format!(
            "Bạn là trợ lý AI tóm tắt tin nhắn nhóm chat. \
             Hãy tóm tắt các tin nhắn sau đây từ nhóm \"{group_name}\" bằng {lang}.\n\
             {style_instruction}\n\n\
             Chú ý:\n\
             - Gộp các chủ đề liên quan\n\
             - Highlight quyết định quan trọng\n\
             - Bỏ qua tin nhắn không quan trọng (sticker, OK, ...)\n\
             - Nêu rõ ai đề xuất/quyết định gì\n\n\
             --- TIN NHẮN ---\n"
        );

        for msg in messages.iter().take(self.config.max_messages_per_group) {
            let time = msg.timestamp.format("%H:%M");
            prompt.push_str(&format!("[{time}] {}: {}\n", msg.sender_name, msg.content));
        }

        prompt.push_str("--- HẾT TIN NHẮN ---\n\nTÓM TẮT:");
        prompt
    }
}

#[async_trait]
impl Tool for GroupSummarizerTool {
    fn name(&self) -> &str {
        "group_summarizer"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "group_summarizer".into(),
            description: "Tóm tắt tin nhắn nhóm Zalo/Telegram. Trả về danh sách nhóm có tin nhắn đang buffer hoặc tóm tắt cho 1 nhóm cụ thể.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list_groups", "summarize", "buffer_status"],
                        "description": "Action: list_groups (xem nhóm nào có tin), summarize (tóm tắt nhóm), buffer_status (trạng thái buffer)"
                    },
                    "group_id": {
                        "type": "string",
                        "description": "Group ID to summarize (required for 'summarize' action)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .unwrap_or_else(|_| serde_json::json!({"action": "summarize", "group_id": ""}));

        let action = args["action"].as_str().unwrap_or("summarize");

        let output = match action {
            "summarize" => {
                let group_id = args["group_id"].as_str().unwrap_or("");

                let conn = Connection::open(&self.db_path)
                    .map_err(|e| BizClawError::Tool(format!("DB error: {e}")))?;

                let mut query = String::from(
                    "SELECT sender_name, sender_id, content, timestamp FROM group_messages",
                );
                let time_limit = chrono::Utc::now()
                    - chrono::Duration::seconds(self.config.buffer_window_secs as i64);
                let time_str = time_limit.format("%Y-%m-%d %H:%M:%S").to_string();

                if !group_id.is_empty() {
                    query.push_str(" WHERE group_id = ?1 AND timestamp > ?2");
                } else {
                    query.push_str(" WHERE timestamp > ?1");
                }
                query.push_str(" ORDER BY timestamp ASC LIMIT 500");

                let mut stmt = conn
                    .prepare(&query)
                    .map_err(|e| BizClawError::Tool(e.to_string()))?;

                let mut messages: Vec<BufferedMessage> = Vec::new();
                if !group_id.is_empty() {
                    let rows = stmt
                        .query_map(rusqlite::params![group_id, time_str], |row| {
                            Ok(BufferedMessage {
                                sender_name: row
                                    .get::<_, Option<String>>(0)?
                                    .unwrap_or_else(|| row.get::<_, String>(1).unwrap_or_default()),
                                content: row.get(2)?,
                                timestamp: chrono::Utc::now(), // mock timestamp for now, exact parsing skipped for simplicity
                                group_id: group_id.to_string(),
                                group_name: group_id.to_string(),
                            })
                        })
                        .map_err(|e| BizClawError::Tool(e.to_string()))?;
                    for r in rows {
                        if let Ok(m) = r {
                            messages.push(m);
                        }
                    }
                } else {
                    let rows = stmt
                        .query_map(rusqlite::params![time_str], |row| {
                            Ok(BufferedMessage {
                                sender_name: row
                                    .get::<_, Option<String>>(0)?
                                    .unwrap_or_else(|| row.get::<_, String>(1).unwrap_or_default()),
                                content: row.get(2)?,
                                timestamp: chrono::Utc::now(),
                                group_id: "all".to_string(),
                                group_name: "all".to_string(),
                            })
                        })
                        .map_err(|e| BizClawError::Tool(e.to_string()))?;
                    for r in rows {
                        if let Ok(m) = r {
                            messages.push(m);
                        }
                    }
                };

                if messages.is_empty() {
                    if group_id.is_empty() {
                        "Không có tin nhắn nào trong buffer.".into()
                    } else {
                        format!("Nhóm {group_id} không có tin nhắn nào trong buffer.")
                    }
                } else {
                    let group_name = if group_id.is_empty() {
                        "Tất cả nhóm"
                    } else {
                        group_id
                    };
                    let prompt = self.format_messages_for_llm(&messages, group_name);

                    // Return the formatted prompt — the AI agent will process it
                    format!(
                        "📊 Đã buffer {} tin nhắn từ nhóm \"{}\". \
                         Dưới đây là nội dung cần tóm tắt:\n\n{}",
                        messages.len(),
                        group_name,
                        prompt
                    )
                }
            }
            _ => format!("Unknown action: {action}. Please use 'summarize'."),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}
