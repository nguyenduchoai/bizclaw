//! Zalo Group Summarizer Tool — buffer group messages and summarize with LLM.
//!
//! Monitors Zalo group chats, buffers messages over a configurable time window,
//! then uses the AI provider to generate a summary.

use async_trait::async_trait;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use bizclaw_core::error::{BizClawError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

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

fn default_buffer_window() -> u64 { 3600 } // 1 hour
fn default_max_messages() -> usize { 200 }
fn default_language() -> String { "vi".into() }
fn default_style() -> String { "bullet_points".into() }

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

/// Message buffer — stores messages per group.
#[derive(Debug, Clone, Default)]
pub struct MessageBuffer {
    /// group_id -> Vec<BufferedMessage>
    groups: Arc<Mutex<HashMap<String, Vec<BufferedMessage>>>>,
}

impl MessageBuffer {
    pub fn new() -> Self {
        Self {
            groups: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a message to the buffer.
    pub fn push(&self, msg: BufferedMessage) {
        let mut groups = self.groups.lock().unwrap();
        groups.entry(msg.group_id.clone())
            .or_default()
            .push(msg);
    }

    /// Get and clear messages for a specific group.
    pub fn drain_group(&self, group_id: &str) -> Vec<BufferedMessage> {
        let mut groups = self.groups.lock().unwrap();
        groups.remove(group_id).unwrap_or_default()
    }

    /// Get all group IDs with buffered messages.
    pub fn group_ids(&self) -> Vec<String> {
        self.groups.lock().unwrap().keys().cloned().collect()
    }

    /// Get message count for a group.
    pub fn count(&self, group_id: &str) -> usize {
        self.groups.lock().unwrap()
            .get(group_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get total message count across all groups.
    pub fn total_count(&self) -> usize {
        self.groups.lock().unwrap()
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Prune old messages beyond the buffer window.
    pub fn prune(&self, max_age_secs: u64) {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs as i64);
        let mut groups = self.groups.lock().unwrap();
        for messages in groups.values_mut() {
            messages.retain(|m| m.timestamp > cutoff);
        }
        groups.retain(|_, v| !v.is_empty());
    }
}

/// Zalo Group Summarizer tool — generates summaries from buffered messages.
pub struct GroupSummarizerTool {
    buffer: MessageBuffer,
    config: SummarizerConfig,
}

impl GroupSummarizerTool {
    pub fn new(config: SummarizerConfig) -> Self {
        Self {
            buffer: MessageBuffer::new(),
            config,
        }
    }

    pub fn with_buffer(buffer: MessageBuffer, config: SummarizerConfig) -> Self {
        Self { buffer, config }
    }

    /// Get the shared message buffer.
    pub fn buffer(&self) -> &MessageBuffer {
        &self.buffer
    }

    /// Format messages into a prompt for the LLM.
    fn format_messages_for_llm(&self, messages: &[BufferedMessage], group_name: &str) -> String {
        let lang = if self.config.language == "vi" { "tiếng Việt" } else { "English" };
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
            prompt.push_str(&format!(
                "[{time}] {}: {}\n",
                msg.sender_name, msg.content
            ));
        }

        prompt.push_str("--- HẾT TIN NHẮN ---\n\nTÓM TẮT:");
        prompt
    }
}

#[async_trait]
impl Tool for GroupSummarizerTool {
    fn name(&self) -> &str { "group_summarizer" }

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
            .unwrap_or_else(|_| serde_json::json!({"action": "buffer_status"}));

        let action = args["action"].as_str().unwrap_or("buffer_status");

        let output = match action {
            "list_groups" => {
                let group_ids = self.buffer.group_ids();
                if group_ids.is_empty() {
                    "Không có nhóm nào có tin nhắn đang buffer.".into()
                } else {
                    let mut out = format!("📋 {} nhóm có tin nhắn đang buffer:\n\n", group_ids.len());
                    for gid in &group_ids {
                        let count = self.buffer.count(gid);
                        out.push_str(&format!("  • {gid}: {count} tin nhắn\n"));
                    }
                    out
                }
            }
            "summarize" => {
                let group_id = args["group_id"].as_str()
                    .ok_or_else(|| BizClawError::Tool("Missing group_id".into()))?;

                let messages = self.buffer.drain_group(group_id);
                if messages.is_empty() {
                    format!("Nhóm {group_id} không có tin nhắn nào trong buffer.")
                } else {
                    let group_name = messages.first()
                        .map(|m| m.group_name.as_str())
                        .unwrap_or(group_id);

                    let prompt = self.format_messages_for_llm(&messages, group_name);

                    // Return the formatted prompt — the AI agent will process it
                    format!(
                        "📊 Đã buffer {} tin nhắn từ nhóm \"{}\". \
                         Dưới đây là nội dung cần tóm tắt:\n\n{}",
                        messages.len(), group_name, prompt
                    )
                }
            }
            "buffer_status" => {
                let total = self.buffer.total_count();
                let groups = self.buffer.group_ids().len();
                format!(
                    "📊 Buffer: {total} tin nhắn từ {groups} nhóm\n\
                     ⏰ Window: {}s\n\
                     📝 Style: {}",
                    self.config.buffer_window_secs,
                    self.config.summary_style
                )
            }
            _ => format!("Unknown action: {action}"),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}
