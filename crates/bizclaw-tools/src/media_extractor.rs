//! Media Extractor Tool
//! Downloads videos/images from social platforms (TikTok, FB, IG, X, YT) using yt-dlp.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub original_url: String,
    pub title: String,
    pub description: String,
    pub filepath: String,
    pub ext: String,
}

pub struct MediaExtractorTool {}

impl MediaExtractorTool {
    pub fn new() -> Self {
        std::fs::create_dir_all("/tmp/bizclaw_media").ok();
        Self {}
    }

    pub async fn extract(&self, url: &str) -> Result<MediaMetadata> {
        // 1. Get JSON metadata
        let output = Command::new("yt-dlp")
            .arg("-j")
            .arg(url)
            .output()
            .await
            .map_err(|e| BizClawError::Tool(format!("yt-dlp execution failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BizClawError::Tool(format!(
                "Failed to parse media: {}",
                stderr
            )));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| BizClawError::Tool(format!("JSON parse error from yt-dlp: {e}")))?;

        let title = parsed["title"].as_str().unwrap_or("Untitled").to_string();
        let description = parsed["description"].as_str().unwrap_or("").to_string();
        let ext = parsed["ext"].as_str().unwrap_or("mp4").to_string();
        let id = parsed["id"].as_str().unwrap_or("unknown_id").to_string();

        let filepath = format!("/tmp/bizclaw_media/{}.{}", id, ext);

        // 2. Download media file
        let dl_output = Command::new("yt-dlp")
            .arg("-f")
            .arg("bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best")
            .arg("-o")
            .arg(&filepath)
            .arg(url)
            .output()
            .await
            .map_err(|e| BizClawError::Tool(format!("Failed to download media: {e}")))?;

        if !dl_output.status.success() {
            let stderr = String::from_utf8_lossy(&dl_output.stderr);
            return Err(BizClawError::Tool(format!("Download failed: {}", stderr)));
        }

        Ok(MediaMetadata {
            original_url: url.to_string(),
            title,
            description,
            filepath,
            ext,
        })
    }
}

#[derive(Deserialize)]
struct ExtractorReq {
    url: String,
}

#[async_trait]
impl Tool for MediaExtractorTool {
    fn name(&self) -> &str {
        "media_extractor"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "media_extractor".into(),
            description: "Takes a URL from TikTok, YouTube, Facebook, Twitter, or Instagram and downloads the high-quality video/image to a local path, returning metadata (title, description, local path) for reposting.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Original URL to download from"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let req: ExtractorReq = serde_json::from_str(args)
            .map_err(|e| BizClawError::Tool(format!("Invalid arguments: {e}")))?;

        match self.extract(&req.url).await {
            Ok(meta) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!(
                    "✅ Media downloaded successfully!\nFile: {}\nTitle: {}\nDescription: {}",
                    meta.filepath, meta.title, meta.description
                ),
                success: true,
            }),
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: e.to_string(),
                success: false,
            }),
        }
    }
}
