//! Facebook Marketing Tools - Wrapper for Marketing Agent
//!
//! Provides Facebook posting and inbox collection capabilities for the marketing agent.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct FacebookPostTool {
    poster: Arc<RwLock<Option<bizclaw_social::FacebookPoster>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FacebookPostArgs {
    action: String,
    content: String,
    image_url: Option<String>,
    agent_name: Option<String>,
    scheduled_time: Option<String>,
}

impl FacebookPostTool {
    pub fn new() -> Self {
        Self {
            poster: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn configure(&self, page_id: &str, access_token: &str, agent_name: &str) -> Result<()> {
        let config = bizclaw_social::FacebookPosterConfig {
            page_id: page_id.to_string(),
            access_token: access_token.to_string(),
            agent_name: agent_name.to_string(),
            auto_retry: true,
            max_retries: 3,
            retry_delay_secs: 60,
        };

        let poster = bizclaw_social::FacebookPoster::new();
        poster.register_account(config).await?;

        let mut guard = self.poster.write().await;
        *guard = Some(poster);
        Ok(())
    }
}

impl Default for FacebookPostTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FacebookPostTool {
    fn name(&self) -> &str {
        "facebook_post"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "facebook_post".into(),
            description: r#"Đăng bài lên Facebook Page cho Marketing Agent.

Actions:
- post_now: Đăng ngay lập tức
- schedule: Lập lịch đăng bài
- get_status: Kiểm tra trạng thái
- cancel: Hủy bài đã lên lịch

Args:
- action: post_now|schedule|get_status|cancel
- content: Nội dung bài viết
- image_url: URL hình ảnh (tùy chọn)
- scheduled_time: ISO timestamp cho schedule (vd: "2024-12-25T10:00:00Z")
- agent_name: Tên agent (mặc định: marketing)"#.into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["post_now", "schedule", "get_status", "cancel"],
                        "description": "Hành động cần thực hiện"
                    },
                    "content": {
                        "type": "string",
                        "description": "Nội dung bài viết"
                    },
                    "image_url": {
                        "type": "string",
                        "description": "URL hình ảnh đính kèm"
                    },
                    "scheduled_time": {
                        "type": "string",
                        "description": "Thời gian lên lịch (ISO 8601)"
                    },
                    "agent_name": {
                        "type": "string",
                        "description": "Tên agent (mặc định: marketing)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let args: FacebookPostArgs = serde_json::from_str(args)
            .map_err(|e| BizClawError::Tool(format!("Invalid args: {e}")))?;

        let poster_guard = self.poster.read().await;
        let poster = poster_guard.as_ref()
            .context("Facebook chưa được cấu hình. Gọi configure trước.")?;

        match args.action.as_str() {
            "post_now" => {
                let agent = args.agent_name.unwrap_or_else(|| "marketing".to_string());
                let result = poster.post_now(&agent, &args.content, args.image_url.as_deref()).await
                    .map_err(|e| BizClawError::Tool(e.to_string()))?;
                
                Ok(ToolResult {
                    tool_call_id: "facebook_post".to_string(),
                    output: format!("✅ Đã đăng bài thành công!\nID: {}\nURL: {}", result.post_id, result.permalink_url),
                    success: true,
                })
            }
            
            "schedule" => {
                let agent = args.agent_name.unwrap_or_else(|| "marketing".to_string());
                let time = args.scheduled_time
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|| Utc::now() + Duration::hours(1));
                
                let post_id = poster.schedule_post(&agent, args.content, args.image_url, time).await
                    .map_err(|e| BizClawError::Tool(e.to_string()))?;
                
                Ok(ToolResult {
                    tool_call_id: "facebook_post".to_string(),
                    output: format!("✅ Đã lên lịch đăng bài!\nID: {}\nThời gian: {}", post_id, time),
                    success: true,
                })
            }
            
            "get_status" => {
                let posts = poster.get_scheduled_posts(args.agent_name.as_deref()).await;
                let output = if posts.is_empty() {
                    "Không có bài nào được lên lịch".to_string()
                } else {
                    posts.iter().map(|p| {
                        format!("📋 {} - {} - {:?}", p.id, p.status == bizclaw_social::PostStatus::Scheduled, p.scheduled_time)
                    }).collect::<Vec<_>>().join("\n")
                };
                
                Ok(ToolResult {
                    tool_call_id: "facebook_post".to_string(),
                    output,
                    success: true,
                })
            }
            
            "cancel" => {
                poster.cancel_post(&args.content).await
                    .map_err(|e| BizClawError::Tool(e.to_string()))?;
                
                Ok(ToolResult {
                    tool_call_id: "facebook_post".to_string(),
                    output: format!("✅ Đã hủy bài: {}", args.content),
                    success: true,
                })
            }
            
            _ => Err(BizClawError::Tool(format!("Action không hợp lệ: {}", args.action)))
        }
    }
}

pub fn new() -> Box<dyn Tool> {
    Box::new(FacebookPostTool::new())
}
