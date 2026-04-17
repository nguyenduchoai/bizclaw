//! OmniChannel types - simplified

pub mod customer360;
pub mod dedupe;

pub use customer360::{Customer360, Customer360Manager};
pub use dedupe::{DedupeEngine, DedupeMatch, DedupeThresholds, MatchType};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Facebook,
    FacebookMessenger,
    Zalo,
    ZaloOA,
    Instagram,
    InstagramDM,
    WhatsApp,
    Telegram,
    Viber,
    LiveChat,
    Website,
    Email,
    SMS,
    Shopee,
    Lazada,
    TikTok,
    Voocchat,
    Other(String),
}

impl Channel {
    pub fn code(&self) -> &str {
        match self {
            Channel::Facebook => "fb",
            Channel::FacebookMessenger => "fb_msg",
            Channel::Zalo => "zalo",
            Channel::ZaloOA => "zalo_oa",
            Channel::Instagram => "ig",
            Channel::InstagramDM => "ig_dm",
            Channel::WhatsApp => "wa",
            Channel::Telegram => "tg",
            Channel::Viber => "viber",
            Channel::LiveChat => "livechat",
            Channel::Website => "web",
            Channel::Email => "email",
            Channel::SMS => "sms",
            Channel::Shopee => "shopee",
            Channel::Lazada => "lazada",
            Channel::TikTok => "tiktok",
            Channel::Voocchat => "voocchat",
            Channel::Other(name) => name,
        }
    }

    pub fn supports_realtime(&self) -> bool {
        matches!(
            self,
            Channel::FacebookMessenger
                | Channel::Zalo
                | Channel::ZaloOA
                | Channel::InstagramDM
                | Channel::WhatsApp
                | Channel::Telegram
                | Channel::Viber
                | Channel::LiveChat
                | Channel::Voocchat
        )
    }

    pub fn supports_channel_id(&self) -> bool {
        matches!(
            self,
            Channel::Facebook
                | Channel::FacebookMessenger
                | Channel::Instagram
                | Channel::InstagramDM
                | Channel::Zalo
                | Channel::ZaloOA
                | Channel::WhatsApp
                | Channel::Telegram
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAccount {
    pub id: String,
    pub channel: Channel,
    pub account_id: String,
    pub account_name: String,
    pub account_avatar: Option<String>,
    pub is_connected: bool,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelContact {
    pub channel: Channel,
    pub channel_contact_id: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub phone: Vec<String>,
    pub email: Vec<String>,
    pub address: Option<String>,
    pub dob: Option<DateTime<Utc>>,
    pub gender: Option<String>,
    pub is_verified: bool,
    pub metadata: HashMap<String, String>,
    pub linked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedContact {
    pub id: String,
    pub primary_name: String,
    pub channels: Vec<ChannelContact>,
    pub merged_ids: Vec<String>,
    pub merge_confidence: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: String,
    pub contact_id: String,
    pub channel: Channel,
    pub interaction_type: InteractionType,
    pub direction: Direction,
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub metadata: HashMap<String, String>,
    pub sentiment: Option<Sentiment>,
    pub intent: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    Message,
    Comment,
    Review,
    Call,
    Email,
    Sms,
    Order,
    Payment,
    Refund,
    Support,
    Feedback,
    Survey,
    Post,
    Share,
    Like,
    Follow,
    Visit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    Inbound,
    Outbound,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub attachment_type: AttachmentType,
    pub url: String,
    pub thumbnail: Option<String>,
    pub filename: Option<String>,
    pub mime_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttachmentType {
    Image,
    Video,
    Audio,
    File,
    Sticker,
    Gif,
    Location,
    Contact,
    Poll,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub contact_id: String,
    pub channel: Channel,
    pub order_id: String,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub currency: String,
    pub status: TransactionStatus,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub product_id: String,
    pub product_name: String,
    pub quantity: u32,
    pub unit_price: f64,
    pub total_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicket {
    pub id: String,
    pub contact_id: String,
    pub channel: Channel,
    pub subject: String,
    pub description: String,
    pub priority: TicketPriority,
    pub status: TicketStatus,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TicketStatus {
    Open,
    InProgress,
    Pending,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub contact_id: String,
    pub channel: Channel,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub reply: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub total_interactions: u64,
    pub interactions_this_month: u64,
    pub avg_response_time_minutes: f32,
    pub last_interaction_at: Option<DateTime<Utc>>,
    pub preferred_channel: Option<Channel>,
    pub active_hours: Vec<u8>,
    pub active_days: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerPreferences {
    pub language: String,
    pub notification_preferences: HashMap<String, bool>,
    pub marketing_consent: bool,
    pub interests: Vec<String>,
    pub categories: Vec<String>,
    pub price_range: Option<(f64, f64)>,
    pub payment_method: Option<String>,
    pub shipping_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifetimeValue {
    pub total_spent: f64,
    pub total_orders: u64,
    pub avg_order_value: f64,
    pub first_purchase: Option<DateTime<Utc>>,
    pub last_purchase: Option<DateTime<Utc>>,
    pub predicted_12m_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextBestAction {
    pub action_type: NBAType,
    pub description: String,
    pub confidence: f32,
    pub reason: String,
    pub channel: Option<Channel>,
    pub urgency: NBAPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NBAType {
    Upsell,
    CrossSell,
    ReEngage,
    Retention,
    WinBack,
    Nurture,
    Support,
    Survey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NBAPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSummary {
    pub total_interactions: u64,
    pub last_interaction: Option<DateTime<Utc>>,
    pub unread_count: u32,
    pub open_tickets: u32,
    pub sentiment_breakdown: HashMap<String, u64>,
}

impl Default for ChannelSummary {
    fn default() -> Self {
        Self {
            total_interactions: 0,
            last_interaction: None,
            unread_count: 0,
            open_tickets: 0,
            sentiment_breakdown: HashMap::new(),
        }
    }
}
