use crate::{ImageGenParams, ImageGenerator, MediaResponse, VideoGenParams, VideoGenerator};
use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

pub struct MiniMaxMediaProvider {
    api_key: String,
    group_id: String,
    client: Client,
}

impl MiniMaxMediaProvider {
    pub fn new(api_key: String, group_id: String) -> Self {
        Self {
            api_key,
            group_id,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ImageGenerator for MiniMaxMediaProvider {
    async fn generate_image(&self, params: &ImageGenParams) -> Result<Vec<MediaResponse>> {
        let url = format!(
            "https://api.minimax.io/v1/text_to_image?GroupId={}",
            self.group_id
        );

        let body = json!({
            "prompt": params.prompt,
            "model": params.model,
            "width": params.width,
            "height": params.height,
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| BizClawError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(BizClawError::Provider(format!(
                "MiniMax image gen failed: {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| BizClawError::Http(e.to_string()))?;

        // Extract URL from response (MiniMax format)
        let image_url = json["base_resp"]["status_msg"]
            .as_str()
            .ok_or_else(|| BizClawError::Provider("Invalid response from MiniMax".into()))?;

        Ok(vec![MediaResponse {
            url: image_url.to_string(),
            raw_data: None,
            mime_type: "image/png".into(),
            provider: "minimax".into(),
        }])
    }
}

#[async_trait]
impl VideoGenerator for MiniMaxMediaProvider {
    async fn generate_video(&self, params: &VideoGenParams) -> Result<MediaResponse> {
        let url = format!(
            "https://api.minimax.io/v1/video_generation?GroupId={}",
            self.group_id
        );

        let body = json!({
            "prompt": params.prompt,
            "model": params.model,
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| BizClawError::Http(e.to_string()))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| BizClawError::Http(e.to_string()))?;
        let task_id = json["task_id"]
            .as_str()
            .ok_or_else(|| BizClawError::Provider("Failed to start video task".into()))?;

        // Polling for completion
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let poll_url = format!(
                "https://api.minimax.io/v1/query_video_generation?GroupId={}&task_id={}",
                self.group_id, task_id
            );

            let poll_resp = self
                .client
                .get(&poll_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await
                .map_err(|e| BizClawError::Http(e.to_string()))?;

            let poll_json: serde_json::Value = poll_resp
                .json()
                .await
                .map_err(|e| BizClawError::Http(e.to_string()))?;

            if poll_json["status"].as_str() == Some("Success") {
                let video_url = poll_json["file_id"]
                    .as_str()
                    .ok_or_else(|| BizClawError::Provider("Missing video URL".into()))?;

                return Ok(MediaResponse {
                    url: video_url.to_string(),
                    raw_data: None,
                    mime_type: "video/mp4".into(),
                    provider: "minimax".into(),
                });
            }
        }

        Err(BizClawError::Provider("Video generation timed out".into()))
    }
}
