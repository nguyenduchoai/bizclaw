use async_trait::async_trait;
use bizclaw_core::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod minimax;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenParams {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub num_images: u32,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoGenParams {
    pub prompt: String,
    pub image_url: Option<String>,
    pub duration: u32, // in seconds
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaResponse {
    pub url: String,
    pub raw_data: Option<Vec<u8>>,
    pub mime_type: String,
    pub provider: String,
}

#[async_trait]
pub trait ImageGenerator: Send + Sync {
    async fn generate_image(&self, params: &ImageGenParams) -> Result<Vec<MediaResponse>>;
}

#[async_trait]
pub trait VideoGenerator: Send + Sync {
    async fn generate_video(&self, params: &VideoGenParams) -> Result<MediaResponse>;
}

pub struct MediaManager {
    image_gen: Option<Box<dyn ImageGenerator>>,
    video_gen: Option<Box<dyn VideoGenerator>>,
}

impl MediaManager {
    pub fn new() -> Self {
        Self {
            image_gen: None,
            video_gen: None,
        }
    }

    pub fn with_image_gen(mut self, gen: Box<dyn ImageGenerator>) -> Self {
        self.image_gen = Some(gen);
        self
    }

    pub fn with_video_gen(mut self, gen: Box<dyn VideoGenerator>) -> Self {
        self.video_gen = Some(gen);
        self
    }

    pub async fn generate_image(&self, params: &ImageGenParams) -> Result<Vec<MediaResponse>> {
        if let Some(gen) = &self.image_gen {
            gen.generate_image(params).await
        } else {
            Err(bizclaw_core::error::BizClawError::Provider(
                "No image generator configured".into(),
            ))
        }
    }

    pub async fn generate_video(&self, params: &VideoGenParams) -> Result<MediaResponse> {
        if let Some(gen) = &self.video_gen {
            gen.generate_video(params).await
        } else {
            Err(bizclaw_core::error::BizClawError::Provider(
                "No video generator configured".into(),
            ))
        }
    }
}
