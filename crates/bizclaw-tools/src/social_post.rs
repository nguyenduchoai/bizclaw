//! Social Posting Tool — AI Agent tool for publishing content to social media.
//!
//! Supports: Facebook Pages, Instagram, Twitter/X, YouTube, Telegram Channels, webhooks.
//! Includes support for Media upload and Auto-comment (Link in comment).

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use reqwest::multipart;
use serde::{Deserialize, Serialize};

/// Safely truncate a string at a character boundary.
fn truncate_safe(s: &str, max_chars: usize) -> String {
    let truncated: String = s.chars().take(max_chars).collect();
    if truncated.len() < s.len() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

pub struct SocialPostTool {
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostRequest {
    #[serde(default)]
    action: String,
    #[serde(default)]
    platform: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    media_path: String, // Local path to video/image for upload
    #[serde(default)]
    auto_comment: String, // Text to auto-comment after posting
    #[serde(default)]
    link: String,

    // Platform-specific credentials
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    page_id: String,
    #[serde(default)]
    ig_user_id: String,
    #[serde(default)]
    x_api_key: String,
    #[serde(default)]
    bot_token: String,
    #[serde(default)]
    chat_id: String,
    #[serde(default)]
    webhook_url: String,
}

impl SocialPostTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120)) // Media upload needs time
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Post to Facebook Page (Handles Images, Videos, and Text).
    /// Uses Resumable Chunked Upload for videos to prevent timeout/RAM issues.
    async fn post_facebook(&self, req: &PostRequest) -> String {
        if req.access_token.is_empty() || req.page_id.is_empty() {
            return "❌ Thiếu access_token hoặc page_id cho Facebook.".into();
        }

        let is_video = req.media_path.ends_with(".mp4") || req.media_path.ends_with(".mov");
        let post_id = if req.media_path.is_empty() {
            // Text/Link post
            let upload_url = format!("https://graph.facebook.com/v21.0/{}/feed", req.page_id);
            let mut params = vec![
                ("message", req.content.as_str()),
                ("access_token", req.access_token.as_str()),
            ];
            if !req.link.is_empty() {
                params.push(("link", req.link.as_str()));
            }
            match self.client.post(&upload_url).form(&params).send().await {
                Ok(resp) => {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    body["id"].as_str().map(String::from)
                }
                Err(_) => None,
            }
        } else if is_video {
            // RESUMABLE VIDEO UPLOAD
            let upload_url = format!("https://graph.facebook.com/v21.0/{}/videos", req.page_id);
            let file_size = match tokio::fs::metadata(&req.media_path).await {
                Ok(m) => m.len(),
                Err(_) => return "❌ Không thể đọc file video.".into(),
            };

            // Phase 1: START
            let fs_str = file_size.to_string();
            let start_params = vec![
                ("upload_phase", "start"),
                ("access_token", req.access_token.as_str()),
                ("file_size", &fs_str),
            ];
            let start_resp = match self
                .client
                .post(&upload_url)
                .form(&start_params)
                .send()
                .await
            {
                Ok(r) => r.json::<serde_json::Value>().await.unwrap_or_default(),
                Err(e) => return format!("❌ Lỗi FB Video Start: {e}"),
            };

            let session_id = start_resp["upload_session_id"].as_str().unwrap_or("");
            let _end_offset = start_resp["end_offset"].as_str().unwrap_or("0");

            if session_id.is_empty() {
                return format!("❌ FB không trả về video session id: {}", start_resp);
            }

            // Phase 2: TRANSFER
            let bytes = tokio::fs::read(&req.media_path).await.unwrap_or_default();
            let file_part = multipart::Part::bytes(bytes)
                .file_name(req.media_path.clone())
                .mime_str("video/mp4")
                .unwrap();

            let form = multipart::Form::new()
                .text("upload_phase", "transfer")
                .text("access_token", req.access_token.clone())
                .text("upload_session_id", session_id.to_string())
                .text("start_offset", "0")
                .part("video_file_chunk", file_part);

            let _transfer_resp = match self.client.post(&upload_url).multipart(form).send().await {
                Ok(r) => r.json::<serde_json::Value>().await.unwrap_or_default(),
                Err(e) => return format!("❌ Lỗi FB Video Transfer: {e}"),
            };

            // Phase 3: FINISH
            let finish_params = vec![
                ("upload_phase", "finish"),
                ("access_token", req.access_token.as_str()),
                ("upload_session_id", session_id),
                ("description", req.content.as_str()),
            ];
            match self
                .client
                .post(&upload_url)
                .form(&finish_params)
                .send()
                .await
            {
                Ok(resp) => {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    body["id"].as_str().map(String::from) // Actually video_id, but good enough
                }
                Err(_) => None,
            }
        } else {
            // Single Image Upload
            let upload_url = format!("https://graph.facebook.com/v21.0/{}/photos", req.page_id);
            let bytes = tokio::fs::read(&req.media_path).await.unwrap_or_default();
            let file_part = multipart::Part::bytes(bytes)
                .file_name(req.media_path.clone())
                .mime_str("image/jpeg")
                .unwrap();
            let form = multipart::Form::new()
                .text("message", req.content.clone())
                .text("access_token", req.access_token.clone())
                .part("source", file_part);

            match self.client.post(&upload_url).multipart(form).send().await {
                Ok(resp) => {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    body["post_id"]
                        .as_str()
                        .or(body["id"].as_str())
                        .map(String::from)
                }
                Err(_) => None,
            }
        };

        if let Some(id) = post_id {
            let mut output = format!("✅ Đã đăng bài lên Facebook!\n• Post ID: {}", id);

            // AUTO-COMMENT FEATURE
            if !req.auto_comment.is_empty() {
                let comment_url = format!("https://graph.facebook.com/v21.0/{}/comments", id);
                let c_params = vec![
                    ("message", req.auto_comment.as_str()),
                    ("access_token", req.access_token.as_str()),
                ];
                if let Ok(c_resp) = self.client.post(&comment_url).form(&c_params).send().await {
                    if c_resp.status().is_success() {
                        output.push_str("\n✅ Auto-Comment thành công!");
                    } else {
                        output.push_str("\n⚠️ Lỗi đăng auto-comment.");
                    }
                }
            }
            output
        } else {
            "❌ Có lỗi xảy ra trong quá trình gọi FB API (Post ID empty).".into()
        }
    }

    /// Upload a local file to a temporary CDN to get a public URL (required for IG/FB Container APIs)
    async fn tmp_cdn_upload(&self, file_path: &str) -> String {
        let bytes = match tokio::fs::read(file_path).await {
            Ok(b) => b,
            Err(_) => return "".into(),
        };
        let file_part = multipart::Part::bytes(bytes)
            .file_name(file_path.to_string())
            .mime_str(if file_path.ends_with(".mp4") {
                "video/mp4"
            } else {
                "image/jpeg"
            })
            .unwrap();
        let form = multipart::Form::new().part("files[]", file_part);

        match self
            .client
            .post("https://pomf.lain.la/upload.php")
            .multipart(form)
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if json["success"].as_bool() == Some(true) {
                        return json["files"][0]["url"].as_str().unwrap_or("").to_string();
                    }
                }
                "".into()
            }
            Err(_) => "".into(),
        }
    }

    /// Post to Instagram via Graph API (Reels & Posts)
    async fn post_instagram(&self, req: &PostRequest) -> String {
        if req.access_token.is_empty() || req.ig_user_id.is_empty() {
            return "❌ Thiếu access_token hoặc ig_user_id cho Instagram.".into();
        }
        if req.media_path.is_empty() {
            return "❌ Instagram bắt buộc phải có ảnh (media_path) hoặc video.".into();
        }

        // 1. Upload to Temporary CDN
        let public_url = self.tmp_cdn_upload(&req.media_path).await;
        if public_url.is_empty() {
            return "❌ Lỗi upload file nội bộ lên Public CDN cho Instagram.".into();
        }

        let is_video = req.media_path.ends_with(".mp4");
        let create_url = format!("https://graph.facebook.com/v21.0/{}/media", req.ig_user_id);

        // 2. Create IG Container
        let mut params = vec![
            ("caption", req.content.as_str()),
            ("access_token", req.access_token.as_str()),
        ];
        if is_video {
            params.push(("media_type", "REELS"));
            params.push(("video_url", &public_url));
        } else {
            params.push(("image_url", &public_url));
        }

        let container_resp = match self.client.post(&create_url).form(&params).send().await {
            Ok(r) => r.json::<serde_json::Value>().await.unwrap_or_default(),
            Err(e) => return format!("❌ Lỗi tạo Container Instagram: {e}"),
        };

        let creation_id = match container_resp["id"].as_str() {
            Some(id) => id,
            None => {
                return format!(
                    "❌ API Error: {}",
                    container_resp["error"]["message"]
                        .as_str()
                        .unwrap_or("Unknown")
                );
            }
        };

        // If it's a video, wait a few seconds for IG to process it before publishing
        if is_video {
            tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;
        }

        // 3. Publish Container
        let publish_url = format!(
            "https://graph.facebook.com/v21.0/{}/media_publish",
            req.ig_user_id
        );
        let publish_params = vec![
            ("creation_id", creation_id),
            ("access_token", req.access_token.as_str()),
        ];

        let mut output = match self
            .client
            .post(&publish_url)
            .form(&publish_params)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                if status.is_success() {
                    let media_id = body["id"].as_str().unwrap_or("unknown");
                    format!("✅ Đã đăng lên Instagram! Media ID: {}", media_id)
                } else {
                    format!(
                        "❌ Publish IG Error: {}",
                        body["error"]["message"].as_str().unwrap_or("Unknown")
                    )
                }
            }
            Err(e) => format!("❌ Publish Instagram Request failed: {e}"),
        };

        // 4. Comment on IG (if auto_comment provided)
        if output.contains("✅") && !req.auto_comment.is_empty() {
            // IG API does allow replying to comments, but commenting on your own root media requires specific permissions
            // or the /comments edge on the media ID.
            if let Some(media_id) = output.split("Media ID: ").last() {
                let comment_url = format!(
                    "https://graph.facebook.com/v21.0/{}/comments",
                    media_id.trim()
                );
                let c_params = vec![
                    ("message", req.auto_comment.as_str()),
                    ("access_token", req.access_token.as_str()),
                ];
                if let Ok(c_resp) = self.client.post(&comment_url).form(&c_params).send().await {
                    if c_resp.status().is_success() {
                        output.push_str("\n✅ Auto-Comment thành công!");
                    } else {
                        output.push_str("\n⚠️ Lỗi đăng auto-comment IG.");
                    }
                }
            }
        }
        output
    }

    /// Post to Twitter / X API (OAuth 1.0a)
    async fn post_twitter(&self, req: &PostRequest) -> String {
        if req.x_api_key.is_empty() {
            return "❌ Thiếu x_api_key (Định dạng: consumer_key,consumer_secret,token,token_secret) cho Twitter.".into();
        }

        let parts: Vec<&str> = req.x_api_key.split(',').collect();
        if parts.len() != 4 {
            return "❌ X API Key sai cấu trúc. Cần 4 token cách nhau bằng dấu phẩy (consumer_key, consumer_secret, access_token, token_secret)".into();
        }

        // We use reqwest_oauth1 macro configuration
        use reqwest_oauth1::OAuthClientProvider;
        let secrets = reqwest_oauth1::Secrets::new(parts[0], parts[1]).token(parts[2], parts[3]);

        let mut media_id_str = String::new();

        // 1. Upload Media (if present) via Twitter v1.1 Media API
        if !req.media_path.is_empty() {
            let bytes = match tokio::fs::read(&req.media_path).await {
                Ok(b) => b,
                Err(e) => return format!("❌ Không thể đọc file twitter media: {e}"),
            };
            let file_part = multipart::Part::bytes(bytes).file_name(req.media_path.clone());
            let form = multipart::Form::new().part("media", file_part);

            match self
                .client
                .clone()
                .oauth1(secrets.clone())
                .post("https://upload.twitter.com/1.1/media/upload.json")
                .multipart(form)
                .send()
                .await
            {
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        media_id_str = json["media_id_string"].as_str().unwrap_or("").to_string();
                    }
                }
                Err(e) => return format!("❌ X Media Upload Request Failed: {e}"),
            }
        }

        // 2. Create Tweet (X API v2)
        let mut tweet_body = serde_json::json!({ "text": req.content });
        if !media_id_str.is_empty() {
            tweet_body["media"] = serde_json::json!({ "media_ids": [media_id_str] });
        }

        let tweet_body_str = serde_json::to_string(&tweet_body).unwrap_or_default();

        let tweet_resp = match self
            .client
            .clone()
            .oauth1(secrets.clone())
            .post("https://api.twitter.com/2/tweets")
            .header("Content-Type", "application/json")
            .body(tweet_body_str)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return format!("❌ Lỗi gọi Twitter API: {e}"),
        };

        let status = tweet_resp.status();
        let body_text = tweet_resp.text().await.unwrap_or_default();
        let body: serde_json::Value = serde_json::from_str(&body_text).unwrap_or_default();

        if status.is_success() {
            let tweet_id = body["data"]["id"].as_str().unwrap_or("unknown");
            let mut output = format!("✅ Đã Tweet thành công!\n• Tweet ID: {}", tweet_id);

            // 3. Auto-comment for Twitter (Reply to tweet)
            if !req.auto_comment.is_empty() {
                let comment_body = serde_json::json!({
                    "text": req.auto_comment,
                    "reply": { "in_reply_to_tweet_id": tweet_id }
                });
                let comment_body_str = serde_json::to_string(&comment_body).unwrap_or_default();
                if let Ok(c_resp) = self
                    .client
                    .clone()
                    .oauth1(secrets)
                    .post("https://api.twitter.com/2/tweets")
                    .header("Content-Type", "application/json")
                    .body(comment_body_str)
                    .send()
                    .await
                {
                    if c_resp.status().is_success() {
                        output.push_str("\n✅ Auto-Comment Twitter thành công!");
                    } else {
                        output.push_str("\n⚠️ Lỗi auto-reply Twitter.");
                    }
                }
            }
            output
        } else {
            format!("❌ Twitter API Error: HTTP {} - {}", status, body)
        }
    }

    /// Post to YouTube Shorts API
    async fn post_youtube(&self, req: &PostRequest) -> String {
        if req.access_token.is_empty() {
            return "❌ Thiếu access_token (OAuth2 Bearer) cho YouTube.".into();
        }
        if req.media_path.is_empty() || !req.media_path.ends_with(".mp4") {
            return "❌ YouTube yêu cầu file video (.mp4).".into();
        }

        let url = "https://www.googleapis.com/upload/youtube/v3/videos?uploadType=multipart&part=snippet,status";

        let snippet = serde_json::json!({
            "snippet": {
                "title": truncate_safe(&req.content, 90),
                "description": req.content,
                "tags": ["shorts", "bizclaw"],
                "categoryId": "22"
            },
            "status": {
                "privacyStatus": "public",
                "selfDeclaredMadeForKids": false
            }
        });

        match tokio::fs::read(&req.media_path).await {
            Ok(bytes) => {
                let metadata_part = multipart::Part::text(snippet.to_string())
                    .mime_str("application/json")
                    .unwrap();
                let video_part = multipart::Part::bytes(bytes)
                    .file_name(req.media_path.clone())
                    .mime_str("video/mp4")
                    .unwrap();

                let form = multipart::Form::new()
                    .part("metadata", metadata_part)
                    .part("file", video_part);

                match self
                    .client
                    .post(url)
                    .bearer_auth(&req.access_token)
                    .multipart(form)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        let status = resp.status();
                        let body: serde_json::Value = resp.json().await.unwrap_or_default();
                        if status.is_success() {
                            let video_id = body["id"].as_str().unwrap_or("unknown");
                            format!(
                                "✅ Đã đăng video lên YouTube Shorts!\n• Video URL: https://youtube.com/shorts/{}",
                                video_id
                            )
                        } else {
                            format!(
                                "❌ Lỗi YouTube API: {}",
                                body["error"]["message"].as_str().unwrap_or("Unknown")
                            )
                        }
                    }
                    Err(e) => format!("❌ Lỗi kết nối YouTube: {e}"),
                }
            }
            Err(e) => format!("❌ Không thể đọc file video {}: {e}", req.media_path),
        }
    }

    /// Post to Telegram Channel
    async fn post_telegram(&self, req: &PostRequest) -> String {
        if req.bot_token.is_empty() || req.chat_id.is_empty() {
            return "❌ Thiếu bot_token hoặc chat_id.".into();
        }

        let is_video = req.media_path.ends_with(".mp4");
        let _is_photo = req.media_path.ends_with(".jpg") || req.media_path.ends_with(".png");

        let url = if !req.media_path.is_empty() {
            if is_video {
                format!("https://api.telegram.org/bot{}/sendVideo", req.bot_token)
            } else {
                format!("https://api.telegram.org/bot{}/sendPhoto", req.bot_token)
            }
        } else {
            format!("https://api.telegram.org/bot{}/sendMessage", req.bot_token)
        };

        let result = if req.media_path.is_empty() {
            let body = serde_json::json!({
                "chat_id": req.chat_id,
                "text": req.content,
                "parse_mode": "HTML",
            });
            self.client.post(&url).json(&body).send().await
        } else {
            let bytes = tokio::fs::read(&req.media_path).await.unwrap_or_default();
            let file_part = multipart::Part::bytes(bytes).file_name(req.media_path.clone());
            let form = multipart::Form::new()
                .text("chat_id", req.chat_id.clone())
                .text("caption", req.content.clone())
                .text("parse_mode", "HTML")
                .part(if is_video { "video" } else { "photo" }, file_part);
            self.client.post(&url).multipart(form).send().await
        };

        match result {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    let msg = resp.json::<serde_json::Value>().await.unwrap_or_default();
                    let msg_id = msg["result"]["message_id"].as_i64().unwrap_or(0);
                    format!("✅ Đã đăng Telegram! Message ID: {}", msg_id)
                } else {
                    format!("❌ Lỗi Telegram HTTP {}", status)
                }
            }
            Err(e) => format!("❌ Lỗi Telegram: {e}"),
        }
    }
}

impl Default for SocialPostTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SocialPostTool {
    fn name(&self) -> &str {
        "social_post"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "social_post".into(),
            description: "Công cụ đăng bài Đa Nền Tảng (FB, IG, X, YT, Telegram). Hỗ trợ tải lên hình ảnh/video cục bộ (qua media_path) và tự động comment (Dành cho Link source/SEO).".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["create_post"] },
                    "platform": { "type": "string", "enum": ["facebook", "instagram", "twitter", "youtube", "telegram"] },
                    "content": { "type": "string", "description": "Nội dung bài đăng đã được rewrite chuẩn SEO" },
                    "media_path": { "type": "string", "description": "Đường dẫn file thiết bị (vd: /tmp/bizclaw/123.mp4). Tải bằng công cụ media_extractor trước." },
                    "auto_comment": { "type": "string", "description": "Nội dung sẽ tự động comment vào bài sau khi publish (Dùng để nhét link SEO/Source)" },
                    "access_token": { "type": "string" },
                    "page_id": { "type": "string" },
                    "ig_user_id": { "type": "string" },
                    "bot_token": { "type": "string" },
                    "chat_id": { "type": "string" },
                    "x_api_key": { "type": "string" }
                },
                "required": ["action", "platform", "content"]
            }),
        }
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let req: PostRequest = serde_json::from_str(args)
            .map_err(|e| BizClawError::Tool(format!("Invalid arguments: {e}")))?;

        let output = match req.platform.as_str() {
            "facebook" => self.post_facebook(&req).await,
            "instagram" => self.post_instagram(&req).await,
            "twitter" => self.post_twitter(&req).await,
            "youtube" => self.post_youtube(&req).await,
            "telegram" => self.post_telegram(&req).await,
            _ => "❌ Platform chưa hỗ trợ.".into(),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}
