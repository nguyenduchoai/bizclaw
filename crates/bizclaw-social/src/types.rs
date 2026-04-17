use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    ZaloOA,
    TikTok,
    Facebook,
    Instagram,
    Shopee,
    #[serde(other)]
    Unknown,
}

impl Platform {
    pub fn code(&self) -> &'static str {
        match self {
            Platform::ZaloOA => "zalo",
            Platform::TikTok => "tiktok",
            Platform::Facebook => "facebook",
            Platform::Instagram => "instagram",
            Platform::Shopee => "shopee",
            Platform::Unknown => "unknown",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "zalo" | "zalo_oa" => Platform::ZaloOA,
            "tiktok" => Platform::TikTok,
            "facebook" | "fb" => Platform::Facebook,
            "instagram" | "ig" => Platform::Instagram,
            "shopee" => Platform::Shopee,
            _ => Platform::Unknown,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    Scheduled,
    Publishing,
    Published,
    Failed,
}

impl fmt::Display for PostStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PostStatus::Draft => write!(f, "draft"),
            PostStatus::Scheduled => write!(f, "scheduled"),
            PostStatus::Publishing => write!(f, "publishing"),
            PostStatus::Published => write!(f, "published"),
            PostStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialContent {
    pub text: String,
    pub hashtags: Vec<String>,
    pub media_urls: Vec<String>,
    pub media_type: Option<MediaType>,
    pub platform: Platform,
    pub link_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
    Album,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledPost {
    pub id: String,
    pub platform: Platform,
    pub content: SocialContent,
    pub scheduled_at: DateTime<Utc>,
    pub status: PostStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl ScheduledPost {
    pub fn new(platform: Platform, content: SocialContent, scheduled_at: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            platform,
            content,
            scheduled_at,
            status: PostStatus::Scheduled,
            created_at: now,
            updated_at: now,
            published_at: None,
            error_message: None,
        }
    }

    pub fn publish(&mut self) {
        self.status = PostStatus::Published;
        self.published_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self, error: String) {
        self.status = PostStatus::Failed;
        self.error_message = Some(error);
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialAccount {
    pub id: String,
    pub platform: Platform,
    pub account_name: String,
    pub account_id: String,
    pub followers_count: u64,
    pub is_verified: bool,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostAnalytics {
    pub post_id: String,
    pub platform: Platform,
    pub views: u64,
    pub clicks: u64,
    pub shares: u64,
    pub comments: u64,
    pub reactions: u64,
    pub reach: u64,
    pub impressions: u64,
    pub engagement_rate: f64,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SocialContentBuilder {
    text: String,
    hashtags: Vec<String>,
    media_urls: Vec<String>,
    media_type: Option<MediaType>,
    platform: Platform,
    link_preview: Option<String>,
}

impl Default for SocialContentBuilder {
    fn default() -> Self {
        Self {
            text: String::new(),
            hashtags: Vec::new(),
            media_urls: Vec::new(),
            media_type: None,
            platform: Platform::Unknown,
            link_preview: None,
        }
    }
}

impl SocialContentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }

    pub fn add_hashtag(mut self, tag: &str) -> Self {
        self.hashtags.push(tag.to_string());
        self
    }

    pub fn hashtags(mut self, tags: Vec<&str>) -> Self {
        self.hashtags = tags.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn add_media(mut self, url: &str, media_type: MediaType) -> Self {
        self.media_urls.push(url.to_string());
        self.media_type = Some(media_type);
        self
    }

    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    pub fn link_preview(mut self, url: &str) -> Self {
        self.link_preview = Some(url.to_string());
        self
    }

    pub fn build(self) -> SocialContent {
        SocialContent {
            text: self.text,
            hashtags: self.hashtags,
            media_urls: self.media_urls,
            media_type: self.media_type,
            platform: self.platform,
            link_preview: self.link_preview,
        }
    }
}

impl SocialContent {
    pub fn builder() -> SocialContentBuilder {
        SocialContentBuilder::new()
    }

    pub fn format_for_platform(&self, platform: Platform) -> String {
        let mut formatted = self.text.clone();

        if !self.hashtags.is_empty() {
            let tag_separator = match platform {
                Platform::ZaloOA => " #",
                Platform::TikTok => " #",
                Platform::Facebook | Platform::Instagram => " #",
                _ => " #",
            };

            let tags = self.hashtags.join(tag_separator);
            formatted = format!("{}\n\n#{}", formatted, tags);
        }

        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_from_code() {
        assert_eq!(Platform::from_code("zalo"), Platform::ZaloOA);
        assert_eq!(Platform::from_code("tiktok"), Platform::TikTok);
        assert_eq!(Platform::from_code("fb"), Platform::Facebook);
    }

    #[test]
    fn test_scheduled_post_lifecycle() {
        let scheduled_at = Utc::now();
        let mut post = ScheduledPost::new(
            Platform::ZaloOA,
            SocialContent::builder().text("Test").build(),
            scheduled_at,
        );

        assert_eq!(post.status, PostStatus::Scheduled);

        post.publish();
        assert_eq!(post.status, PostStatus::Published);
        assert!(post.published_at.is_some());

        let mut failed_post = ScheduledPost::new(
            Platform::TikTok,
            SocialContent::builder().text("Test").build(),
            scheduled_at,
        );

        failed_post.fail("Network error".to_string());
        assert_eq!(post.status, PostStatus::Published);
        assert_eq!(failed_post.error_message, Some("Network error".to_string()));
    }

    #[test]
    fn test_content_format_for_platform() {
        let content = SocialContent::builder()
            .text("Check out our new product!")
            .hashtags(vec!["bizclaw", "startup"])
            .platform(Platform::ZaloOA)
            .build();

        let formatted = content.format_for_platform(Platform::ZaloOA);
        assert!(formatted.contains("#bizclaw"));
        assert!(formatted.contains("#startup"));
    }
}
