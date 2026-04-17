use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub id: String,
    pub title: String,
    pub body: String,
    pub platform: ContentPlatform,
    pub content_type: ContentType,
    pub media: Vec<Media>,
    pub hashtags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub status: ContentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentPlatform {
    Facebook,
    Zalo,
    TikTok,
    Shopee,
    Website,
    Email,
}

impl std::fmt::Display for ContentPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentPlatform::Facebook => write!(f, "facebook"),
            ContentPlatform::Zalo => write!(f, "zalo"),
            ContentPlatform::TikTok => write!(f, "tiktok"),
            ContentPlatform::Shopee => write!(f, "shopee"),
            ContentPlatform::Website => write!(f, "website"),
            ContentPlatform::Email => write!(f, "email"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    Post,
    Story,
    Reel,
    Ad,
    Reply,
    Comment,
    ProductDescription,
    Newsletter,
    Announcement,
    Carousel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentStatus {
    Draft,
    Scheduled,
    Published,
    Failed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub media_type: MediaType,
    pub url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaType {
    Image,
    Video,
    Gif,
    Carousel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTemplate {
    pub id: String,
    pub name: String,
    pub platform: ContentPlatform,
    pub content_type: ContentType,
    pub prompt_template: String,
    pub variables: Vec<TemplateVariable>,
    pub examples: Vec<String>,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentCampaign {
    pub id: String,
    pub name: String,
    pub description: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub platforms: Vec<ContentPlatform>,
    pub content_ids: Vec<String>,
    pub status: CampaignStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CampaignStatus {
    Planning,
    Active,
    Paused,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetrics {
    pub content_id: String,
    pub views: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub clicks: i64,
    pub conversions: i64,
    pub reach: i64,
    pub engagement_rate: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub content_type: ContentType,
    pub platform: ContentPlatform,
    pub topic: String,
    pub tone: Tone,
    pub target_audience: Option<String>,
    pub keywords: Vec<String>,
    pub length: ContentLength,
    pub include_media_suggestion: bool,
    pub custom_variables: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Tone {
    Professional,
    Casual,
    Humorous,
    Inspirational,
    Urgent,
    Educational,
    Promotional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentLength {
    Short,
    Medium,
    Long,
}

impl Default for Tone {
    fn default() -> Self {
        Tone::Professional
    }
}

impl Default for ContentLength {
    fn default() -> Self {
        ContentLength::Medium
    }
}
