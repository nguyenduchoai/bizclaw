use crate::facebook::FacebookClient;
use crate::instagram::InstagramClient;
use crate::tiktok::TikTokClient;
use crate::types::*;
use crate::zalo::ZaloClient;
use anyhow::{Context, Result};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

#[async_trait]
pub trait SocialPoster: Send + Sync {
    async fn post(&self, content: &SocialContent) -> Result<String>;
    async fn get_post_url(&self, post_id: &str) -> Result<String>;
}

#[async_trait]
impl SocialPoster for ZaloClient {
    async fn post(&self, content: &SocialContent) -> Result<String> {
        let formatted = content.format_for_platform(Platform::ZaloOA);
        let user_id = "default";

        let response = self.send_text_message(user_id, &formatted).await?;
        Ok(response.msg_id.unwrap_or_default())
    }

    async fn get_post_url(&self, post_id: &str) -> Result<String> {
        Ok(format!("https://zalo.me/{}?type=post", post_id))
    }
}

#[async_trait]
impl SocialPoster for TikTokClient {
    async fn post(&self, content: &SocialContent) -> Result<String> {
        if content.media_urls.is_empty() {
            anyhow::bail!("TikTok requires a video to post");
        }

        if content.media_urls.len() == 1 {
            let video_id = self
                .publish_video(
                    &content.media_urls[0],
                    &content.text,
                    &content.hashtags.join(" "),
                )
                .await?;
            Ok(video_id)
        } else {
            anyhow::bail!("TikTok only supports single video posts");
        }
    }

    async fn get_post_url(&self, post_id: &str) -> Result<String> {
        Ok(format!("https://www.tiktok.com/@user/video/{}", post_id))
    }
}

#[async_trait]
impl SocialPoster for FacebookClient {
    async fn post(&self, content: &SocialContent) -> Result<String> {
        let formatted = content.format_for_platform(Platform::Facebook);
        let image_url = content.media_urls.first().map(|s| s.as_str());

        let post_id = self.create_post(&formatted, image_url).await?;
        Ok(post_id)
    }

    async fn get_post_url(&self, post_id: &str) -> Result<String> {
        Ok(format!(
            "https://www.facebook.com/permalink.php?story_fbid={}&id=self",
            post_id
        ))
    }
}

#[async_trait]
impl SocialPoster for InstagramClient {
    async fn post(&self, content: &SocialContent) -> Result<String> {
        if content.media_urls.is_empty() {
            anyhow::bail!("Instagram requires an image or video");
        }

        let caption = content.format_for_platform(Platform::Instagram);

        let creation_id = self
            .create_image_post(&content.media_urls[0], &caption)
            .await?;
        let response = self.publish_media(&creation_id).await?;
        Ok(response.id)
    }

    async fn get_post_url(&self, post_id: &str) -> Result<String> {
        Ok(format!("https://www.instagram.com/p/{}", post_id))
    }
}

pub struct SocialAdapter {
    clients: Arc<RwLock<HashMap<Platform, Box<dyn SocialPoster + Send + Sync>>>>,
}

impl SocialAdapter {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_zalo(&self, client: ZaloClient) {
        let mut clients = self.clients.write();
        clients.insert(Platform::ZaloOA, Box::new(client));
    }

    pub fn register_tiktok(&self, client: TikTokClient) {
        let mut clients = self.clients.write();
        clients.insert(Platform::TikTok, Box::new(client));
    }

    pub fn register_facebook(&self, client: FacebookClient) {
        let mut clients = self.clients.write();
        clients.insert(Platform::Facebook, Box::new(client));
    }

    pub fn register_instagram(&self, client: InstagramClient) {
        let mut clients = self.clients.write();
        clients.insert(Platform::Instagram, Box::new(client));
    }

    pub async fn post_to_platform(
        &self,
        platform: Platform,
        content: &SocialContent,
    ) -> Result<String> {
        let clients = self.clients.read();

        let client = clients
            .get(&platform)
            .context(format!("No client registered for platform: {:?}", platform))?;

        client.post(content).await
    }

    pub async fn get_post_url(&self, platform: Platform, post_id: &str) -> Result<String> {
        let clients = self.clients.read();

        let client = clients
            .get(&platform)
            .context(format!("No client registered for platform: {:?}", platform))?;

        client.get_post_url(post_id).await
    }

    pub fn is_platform_registered(&self, platform: Platform) -> bool {
        let clients = self.clients.read();
        clients.contains_key(&platform)
    }

    pub fn registered_platforms(&self) -> Vec<Platform> {
        let clients = self.clients.read();
        clients.keys().cloned().collect()
    }
}

impl Default for SocialAdapter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MultiPlatformPoster {
    adapter: SocialAdapter,
}

impl MultiPlatformPoster {
    pub fn new() -> Self {
        Self {
            adapter: SocialAdapter::new(),
        }
    }

    pub fn with_zalo(mut self, client: ZaloClient) -> Self {
        self.adapter.register_zalo(client);
        self
    }

    pub fn with_tiktok(mut self, client: TikTokClient) -> Self {
        self.adapter.register_tiktok(client);
        self
    }

    pub fn with_facebook(mut self, client: FacebookClient) -> Self {
        self.adapter.register_facebook(client);
        self
    }

    pub fn with_instagram(mut self, client: InstagramClient) -> Self {
        self.adapter.register_instagram(client);
        self
    }

    pub async fn broadcast(&self, content: SocialContent) -> Vec<(Platform, Result<String>)> {
        let mut results = Vec::new();

        for platform in self.adapter.registered_platforms() {
            let result = self.adapter.post_to_platform(platform, &content).await;
            results.push((platform, result));
        }

        results
    }

    pub async fn post_to(
        &self,
        platforms: Vec<Platform>,
        content: SocialContent,
    ) -> Vec<(Platform, Result<String>)> {
        let mut results = Vec::new();

        for platform in platforms {
            let result = self.adapter.post_to_platform(platform, &content).await;
            results.push((platform, result));
        }

        results
    }

    pub fn is_platform_available(&self, platform: Platform) -> bool {
        self.adapter.is_platform_registered(platform)
    }
}

impl Default for MultiPlatformPoster {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ContentAdapter {
    pub platform: Platform,
}

impl ContentAdapter {
    pub fn for_platform(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn adapt_text(&self, content: &str) -> String {
        match self.platform {
            Platform::TikTok => {
                if content.len() > 150 {
                    format!("{}...", &content[..147])
                } else {
                    content.to_string()
                }
            }
            Platform::ZaloOA => content.to_string(),
            Platform::Facebook => {
                if content.len() > 632 {
                    format!("{}...", &content[..629])
                } else {
                    content.to_string()
                }
            }
            Platform::Instagram => {
                if content.len() > 2200 {
                    format!("{}...", &content[..2197])
                } else {
                    content.to_string()
                }
            }
            Platform::Shopee => content.chars().take(500).collect(),
            Platform::Unknown => content.to_string(),
        }
    }

    pub fn adapt_hashtags(&self, hashtags: &[String]) -> Vec<String> {
        let max_tags = match self.platform {
            Platform::TikTok => 10,
            Platform::Instagram => 30,
            Platform::Facebook => 5,
            Platform::ZaloOA => 20,
            Platform::Shopee => 0,
            Platform::Unknown => 10,
        };

        hashtags.iter().take(max_tags).cloned().collect()
    }

    pub fn adapt_media(&self, urls: &[String]) -> Vec<String> {
        let max_media = match self.platform {
            Platform::Instagram => 10,
            Platform::Facebook => 1,
            Platform::TikTok => 1,
            Platform::ZaloOA => 9,
            Platform::Shopee => 8,
            Platform::Unknown => 1,
        };

        urls.iter().take(max_media).cloned().collect()
    }

    pub fn adapt_content(&self, content: &SocialContent) -> SocialContent {
        SocialContent {
            text: self.adapt_text(&content.text),
            hashtags: self.adapt_hashtags(&content.hashtags),
            media_urls: self.adapt_media(&content.media_urls),
            media_type: content.media_type,
            platform: content.platform,
            link_preview: content.link_preview.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_registration() {
        let adapter = SocialAdapter::new();

        assert!(!adapter.is_platform_registered(Platform::ZaloOA));

        let zalo = ZaloClient::new("test_token".to_string());
        adapter.register_zalo(zalo);

        assert!(adapter.is_platform_registered(Platform::ZaloOA));
    }

    #[test]
    fn test_multi_platform_poster() {
        let poster = MultiPlatformPoster::new()
            .with_zalo(ZaloClient::new("zalo_token".to_string()))
            .with_facebook(FacebookClient::new(
                "fb_token".to_string(),
                Some("page_1".to_string()),
            ));

        assert!(poster.is_platform_available(Platform::ZaloOA));
        assert!(poster.is_platform_available(Platform::Facebook));
        assert!(!poster.is_platform_available(Platform::TikTok));
    }

    #[test]
    fn test_content_adapter_tiktok() {
        let adapter = ContentAdapter::for_platform(Platform::TikTok);

        let long_text = "a".repeat(200);
        let adapted = adapter.adapt_text(&long_text);
        assert!(adapted.len() <= 150);

        let hashtags: Vec<String> = (0..15).map(|i| format!("tag{}", i)).collect();
        let adapted_tags = adapter.adapt_hashtags(&hashtags);
        assert!(adapted_tags.len() <= 10);
    }

    #[test]
    fn test_content_adapter_instagram() {
        let adapter = ContentAdapter::for_platform(Platform::Instagram);

        let hashtags: Vec<String> = (0..35).map(|i| format!("tag{}", i)).collect();
        let adapted_tags = adapter.adapt_hashtags(&hashtags);
        assert!(adapted_tags.len() <= 30);
    }
}
