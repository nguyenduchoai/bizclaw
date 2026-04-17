use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct AutoMessageArgs {
    platform: String,
    action: String,
    target: String,
    message: Option<String>,
}

pub struct AutoMessageTool;

impl AutoMessageTool {
    pub fn new() -> Self {
        Self
    }

    async fn send_zalo_mac(&self, target: &str, message: &str) -> Result<String> {
        let script = format!(
            r#"
tell application "Zalo"
    activate
end tell
delay 1
tell application "System Events"
    tell process "Zalo"
        -- Dùng phím tắt Cmd+F để focus ô tìm kiếm
        keystroke "f" using command down
        delay 0.5
        -- Gõ tên người nhận
        keystroke "{}"
        delay 1.5
        -- Nhấn Enter để chọn người đầu tiên
        key code 36
        delay 1.0
        -- Gõ tin nhắn
        keystroke "{}"
        delay 0.5
        -- Nhấn Enter để gửi
        key code 36
    end tell
end tell
            "#,
            target.replace("\"", "\\\""),
            message.replace("\"", "\\\"")
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| BizClawError::Tool(format!("AppleScript error: {}", e)))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(BizClawError::Tool(format!(
                "Zalo automation failed: {}",
                err
            )));
        }

        Ok(format!(
            "✅ Đã gửi Zalo tới '{}' thành công qua UI Macro.",
            target
        ))
    }

    async fn send_messenger_mac(&self, target: &str, message: &str) -> Result<String> {
        let script = format!(
            r#"
tell application "Messenger"
    activate
end tell
delay 1
tell application "System Events"
    tell process "Messenger"
        keystroke "k" using command down
        delay 0.5
        keystroke "{}"
        delay 1.5
        key code 36
        delay 1.0
        keystroke "{}"
        delay 0.5
        key code 36
    end tell
end tell
            "#,
            target.replace("\"", "\\\""),
            message.replace("\"", "\\\"")
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| BizClawError::Tool(format!("AppleScript error: {}", e)))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(BizClawError::Tool(format!(
                "Messenger automation failed: {}",
                err
            )));
        }

        Ok(format!(
            "✅ Đã gửi Messenger tới '{}' thành công qua UI Macro.",
            target
        ))
    }
}

#[async_trait]
impl Tool for AutoMessageTool {
    fn name(&self) -> &str {
        "auto_message"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "auto_message".into(),
            description: "Công cụ cho tự động gửi tin nhắn Zalo/Messenger trên Mac mà KHÔNG vi phạm chính sách API (sử dụng UI Automation). Dùng để Auto-reply hoặc chủ động nhắn tin Broadcast bằng giao diện thật, an toàn 100% cho SME.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "platform": { "type": "string", "enum": ["zalo", "messenger"], "description": "Nền tảng (zalo hoặc messenger)" },
                    "action": { "type": "string", "enum": ["send_message"], "description": "Hành động thực hiện" },
                    "target": { "type": "string", "description": "Tên người nhận (phải khớp chính xác danh bạ) hoặc số điện thoại" },
                    "message": { "type": "string", "description": "Nội dung tin nhắn cần gửi" }
                },
                "required": ["platform", "action", "target", "message"]
            }),
        }
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let parsed: AutoMessageArgs = serde_json::from_str(args)
            .map_err(|e| BizClawError::Tool(format!("Lỗi parse JSON: {e}")))?;

        let out = match parsed.platform.as_str() {
            "zalo" => {
                if parsed.action == "send_message" {
                    self.send_zalo_mac(&parsed.target, parsed.message.as_deref().unwrap_or(""))
                        .await?
                } else {
                    "❌ Hành động không hỗ trợ".into()
                }
            }
            "messenger" => {
                if parsed.action == "send_message" {
                    self.send_messenger_mac(&parsed.target, parsed.message.as_deref().unwrap_or(""))
                        .await?
                } else {
                    "❌ Hành động không hỗ trợ".into()
                }
            }
            _ => format!(
                "❌ Nền tảng {} chưa được hỗ trợ Full UI Automation.",
                parsed.platform
            ),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: out,
            success: true,
        })
    }
}
