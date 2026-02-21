//! Zalo data models (19 types from zca-js).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloUser {
    pub id: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloGroup {
    pub id: String,
    pub name: String,
    pub member_count: u32,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloMessage {
    pub msg_id: String,
    pub thread_id: String,
    pub sender_id: String,
    pub content: ZaloMessageContent,
    pub timestamp: u64,
    pub is_self: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ZaloMessageContent {
    Text(String),
    Attachment(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloReaction {
    pub msg_id: String,
    pub reactor_id: String,
    pub reaction_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloGroupEvent {
    pub group_id: String,
    pub event_type: String,
    pub actor_id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloCatalog {
    pub id: String,
    pub name: String,
    pub products: Vec<ZaloProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloProduct {
    pub id: String,
    pub name: String,
    pub price: Option<f64>,
    pub photo_url: Option<String>,
}
