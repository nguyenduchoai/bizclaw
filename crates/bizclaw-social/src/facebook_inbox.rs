//! # Facebook Inbox Collector - Message aggregation and routing
//!
//! Features:
//! - Webhook integration for real-time updates
//! - Message filtering by keywords, sender, time
//! - Auto-classification and labeling
//! - Route messages to appropriate agents
//! - Secure data storage
//! - Dashboard for management

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxConfig {
    pub page_id: String,
    pub access_token: String,
    pub verify_token: String,
    pub webhook_secret: String,
    pub auto_reply: bool,
    pub routing_enabled: bool,
}

impl Default for InboxConfig {
    fn default() -> Self {
        Self {
            page_id: String::new(),
            access_token: String::new(),
            verify_token: uuid::Uuid::new_v4().to_string(),
            webhook_secret: String::new(),
            auto_reply: false,
            routing_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacebookMessage {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub recipient_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub attachments: Vec<MessageAttachment>,
    pub thread_id: String,
    pub is_read: bool,
    pub labels: Vec<String>,
    pub routed_to: Option<String>,
    pub classification: Option<MessageClassification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAttachment {
    pub id: String,
    pub type_: AttachmentType,
    pub url: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentType {
    Image,
    Video,
    Audio,
    File,
    Location,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageClassification {
    pub category: String,
    pub confidence: f32,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub participant_id: String,
    pub participant_name: String,
    pub last_message: Option<String>,
    pub last_message_time: Option<DateTime<Utc>>,
    pub message_count: u64,
    pub is_customer: bool,
    pub labels: Vec<String>,
    pub assigned_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    pub time: u64,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookChange {
    pub field: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub name: String,
    pub conditions: Vec<RoutingCondition>,
    pub action: RoutingAction,
    pub priority: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCondition {
    pub field: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingAction {
    pub route_to: String,
    pub add_label: Option<String>,
    pub auto_reply: Option<String>,
}

pub struct FacebookInboxCollector {
    config: Arc<RwLock<Option<InboxConfig>>>,
    messages: Arc<RwLock<Vec<FacebookMessage>>>,
    conversations: Arc<RwLock<Vec<Conversation>>>,
    routing_rules: Arc<RwLock<Vec<RoutingRule>>>,
    client: Client,
}

impl FacebookInboxCollector {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(None)),
            messages: Arc::new(RwLock::new(Vec::new())),
            conversations: Arc::new(RwLock::new(Vec::new())),
            routing_rules: Arc::new(RwLock::new(Vec::new())),
            client: Client::new(),
        }
    }

    pub async fn configure(&self, config: InboxConfig) -> Result<()> {
        let mut cfg = self.config.write().await;
        *cfg = Some(config);
        info!("Inbox collector configured");
        Ok(())
    }

    pub async fn verify_webhook(&self, mode: &str, token: &str, challenge: &str) -> Result<String> {
        let cfg = self.config.read().await;
        let config = cfg.as_ref().context("Not configured")?;

        if token == config.verify_token {
            Ok(challenge.to_string())
        } else {
            anyhow::bail!("Invalid verify token")
        }
    }

    pub async fn handle_webhook(&self, event: WebhookEvent) -> Result<Vec<FacebookMessage>> {
        let cfg = self.config.read().await;
        let config = cfg.as_ref().context("Not configured")?;

        let mut new_messages = Vec::new();

        for entry in event.entry {
            for change in entry.changes {
                if change.field == "conversations" || change.field == "messages" {
                    if let Some(msg_data) = change.value.get("message") {
                        let message = self
                            .parse_message(msg_data.clone(), &config.page_id)
                            .await?;
                        let classified = self.classify_message(&message).await;
                        let mut msg = classified;

                        if config.routing_enabled {
                            let route_to = self.apply_routing_rules(&msg).await;
                            if let Some(agent) = route_to {
                                msg.routed_to = Some(agent);
                            }
                        }

                        self.store_message(msg.clone()).await;
                        new_messages.push(msg);
                    }
                }
            }
        }

        Ok(new_messages)
    }

    async fn parse_message(
        &self,
        data: serde_json::Value,
        page_id: &str,
    ) -> Result<FacebookMessage> {
        let id = data["mid"].as_str().unwrap_or("").to_string();
        let sender_id = data["from"]["id"].as_str().unwrap_or("").to_string();
        let sender_name = data["from"]["name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();
        let content = data["message"].as_str().unwrap_or("").to_string();
        let timestamp = data["created_time"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let mut attachments = Vec::new();
        if let Some(attachments_data) = data["attachments"].as_array() {
            for att in attachments_data {
                attachments.push(MessageAttachment {
                    id: att["id"].as_str().unwrap_or("").to_string(),
                    type_: match att["type"].as_str().unwrap_or("") {
                        "image" => AttachmentType::Image,
                        "video" => AttachmentType::Video,
                        "audio" => AttachmentType::Audio,
                        "file" => AttachmentType::File,
                        "location" => AttachmentType::Location,
                        _ => AttachmentType::File,
                    },
                    url: att["url"].as_str().map(String::from),
                    mime_type: att["mime_type"].as_str().map(String::from),
                });
            }
        }

        Ok(FacebookMessage {
            id,
            sender_id,
            sender_name,
            recipient_id: page_id.to_string(),
            content,
            timestamp,
            attachments,
            thread_id: data["thread_id"].as_str().unwrap_or("").to_string(),
            is_read: false,
            labels: Vec::new(),
            routed_to: None,
            classification: None,
        })
    }

    async fn classify_message(&self, message: &FacebookMessage) -> FacebookMessage {
        let content_lower = message.content.to_lowercase();

        let (category, keywords) = self.detect_category(&content_lower);

        FacebookMessage {
            classification: Some(MessageClassification {
                category: category.clone(),
                confidence: 0.85,
                keywords: keywords.clone(),
            }),
            labels: vec![category],
            ..message.clone()
        }
    }

    fn detect_category(&self, content: &str) -> (String, Vec<String>) {
        let keywords_order: Vec<(&str, Vec<&str>)> = vec![
            ("order", vec!["đơn hàng", "mua", "giá", "ship", "đặt"]),
            ("support", vec!["help", "hỗ trợ", "lỗi", "problem", "giúp"]),
            ("complaint", vec!["khiếu nại", "không hài lòng", "tệ", "dở"]),
            ("inquiry", vec!["hỏi", "thắc mắc", "tư vấn", "cho hỏi"]),
            ("feedback", vec!["góp ý", "đề xuất", "cải thiện", "tốt hơn"]),
        ];

        for (category, words) in keywords_order {
            let matches: Vec<&str> = words
                .iter()
                .filter(|w| content.contains(*w))
                .cloned()
                .collect();

            if !matches.is_empty() {
                return (
                    category.to_string(),
                    matches.into_iter().map(String::from).collect(),
                );
            }
        }

        ("general".to_string(), vec![])
    }

    async fn apply_routing_rules(&self, message: &FacebookMessage) -> Option<String> {
        let rules = self.routing_rules.read().await;

        for rule in rules.iter().filter(|r| r.enabled) {
            if self.match_conditions(&rule.conditions, message) {
                return Some(rule.action.route_to.clone());
            }
        }

        None
    }

    fn match_conditions(&self, conditions: &[RoutingCondition], message: &FacebookMessage) -> bool {
        for cond in conditions {
            let matches = match cond.field.as_str() {
                "category" => message
                    .classification
                    .as_ref()
                    .map(|c| c.category == cond.value)
                    .unwrap_or(false),
                "sender_id" => message.sender_id == cond.value,
                "keyword" => message
                    .content
                    .to_lowercase()
                    .contains(&cond.value.to_lowercase()),
                _ => false,
            };

            if !matches {
                return false;
            }
        }
        true
    }

    async fn store_message(&self, message: FacebookMessage) {
        let mut messages = self.messages.write().await;
        messages.push(message.clone());

        let mut conversations = self.conversations.write().await;
        let conv_exists = conversations.iter_mut().find(|c| c.id == message.thread_id);

        if let Some(conv) = conv_exists {
            conv.last_message = Some(message.content.clone());
            conv.last_message_time = Some(message.timestamp);
            conv.message_count += 1;
        } else {
            conversations.push(Conversation {
                id: message.thread_id.clone(),
                participant_id: message.sender_id.clone(),
                participant_name: message.sender_name.clone(),
                last_message: Some(message.content),
                last_message_time: Some(message.timestamp),
                message_count: 1,
                is_customer: true,
                labels: Vec::new(),
                assigned_agent: message.routed_to.clone(),
            });
        }

        debug!("Stored message: {}", message.id);
    }

    pub async fn add_routing_rule(&self, rule: RoutingRule) -> Result<String> {
        let mut rules = self.routing_rules.write().await;
        rules.push(rule.clone());
        info!("Added routing rule: {}", rule.name);
        Ok(rule.id)
    }

    pub async fn get_messages(&self, filters: Option<MessageFilters>) -> Vec<FacebookMessage> {
        let messages = self.messages.read().await;

        match filters {
            Some(f) => messages
                .iter()
                .filter(|m| {
                    if let Some(sender) = &f.sender_id {
                        if &m.sender_id != sender {
                            return false;
                        }
                    }
                    if let Some(keyword) = &f.keyword {
                        if !m.content.to_lowercase().contains(&keyword.to_lowercase()) {
                            return false;
                        }
                    }
                    if let Some(start) = f.from_time {
                        if m.timestamp < start {
                            return false;
                        }
                    }
                    if let Some(end) = f.to_time {
                        if m.timestamp > end {
                            return false;
                        }
                    }
                    if let Some(classification) = &f.classification {
                        if m.classification
                            .as_ref()
                            .map(|c| &c.category != classification)
                            .unwrap_or(true)
                        {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect(),
            None => messages.clone(),
        }
    }

    pub async fn get_conversations(&self) -> Vec<Conversation> {
        self.conversations.read().await.clone()
    }

    pub async fn mark_as_read(&self, message_id: &str) -> Result<()> {
        let mut messages = self.messages.write().await;
        if let Some(msg) = messages.iter_mut().find(|m| m.id == message_id) {
            msg.is_read = true;
        }
        Ok(())
    }

    pub async fn add_label(&self, message_id: &str, label: &str) -> Result<()> {
        let mut messages = self.messages.write().await;
        if let Some(msg) = messages.iter_mut().find(|m| m.id == message_id) {
            if !msg.labels.contains(&label.to_string()) {
                msg.labels.push(label.to_string());
            }
        }
        Ok(())
    }

    pub async fn get_statistics(&self) -> InboxStatistics {
        let messages = self.messages.read().await;
        let conversations = self.conversations.read().await;

        let mut category_counts: HashMap<String, u64> = HashMap::new();
        let mut label_counts: HashMap<String, u64> = HashMap::new();

        for msg in messages.iter() {
            if let Some(class) = &msg.classification {
                *category_counts.entry(class.category.clone()).or_insert(0) += 1;
            }
            for label in &msg.labels {
                *label_counts.entry(label.clone()).or_insert(0) += 1;
            }
        }

        InboxStatistics {
            total_messages: messages.len() as u64,
            unread_messages: messages.iter().filter(|m| !m.is_read).count() as u64,
            total_conversations: conversations.len() as u64,
            category_counts,
            label_counts,
        }
    }

    pub async fn fetch_messages_from_api(&self, limit: u32) -> Result<Vec<FacebookMessage>> {
        let cfg = self.config.read().await;
        let config = cfg.as_ref().context("Not configured")?;

        let url = format!(
            "https://graph.facebook.com/v18.0/{}/conversations?fields=messages{{from,message,created_time,attachments}}&limit={}&access_token={}",
            config.page_id, limit, config.access_token
        );

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct ConversationsResponse {
            data: Vec<ConversationData>,
        }

        #[derive(Deserialize)]
        struct ConversationData {
            id: String,
            messages: MessagesData,
        }

        #[derive(Deserialize)]
        struct MessagesData {
            data: Vec<MessageData>,
        }

        #[derive(Deserialize)]
        struct MessageData {
            id: String,
            from: SenderData,
            message: Option<String>,
            created_time: String,
            attachments: Option<AttachmentsData>,
        }

        #[derive(Deserialize)]
        struct SenderData {
            id: String,
            name: String,
        }

        #[derive(Deserialize)]
        struct AttachmentsData {
            data: Vec<AttachmentData>,
        }

        #[derive(Deserialize)]
        struct AttachmentData {
            id: String,
            #[serde(rename = "type")]
            attachment_type: String,
            url: Option<String>,
        }

        let result: ConversationsResponse = response
            .json()
            .await
            .unwrap_or(ConversationsResponse { data: vec![] });

        let mut messages = Vec::new();

        for conv in result.data {
            for msg_data in conv.messages.data {
                let attachments = msg_data
                    .attachments
                    .as_ref()
                    .map(|a| {
                        a.data
                            .iter()
                            .map(|att| MessageAttachment {
                                id: att.id.clone(),
                                type_: match att.attachment_type.as_str() {
                                    "image" => AttachmentType::Image,
                                    "video" => AttachmentType::Video,
                                    "audio" => AttachmentType::Audio,
                                    _ => AttachmentType::File,
                                },
                                url: att.url.clone(),
                                mime_type: None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let timestamp = chrono::DateTime::parse_from_rfc3339(&msg_data.created_time)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                let msg = FacebookMessage {
                    id: msg_data.id,
                    sender_id: msg_data.from.id,
                    sender_name: msg_data.from.name,
                    recipient_id: config.page_id.clone(),
                    content: msg_data.message.unwrap_or_default(),
                    timestamp,
                    attachments,
                    thread_id: conv.id.clone(),
                    is_read: false,
                    labels: Vec::new(),
                    routed_to: None,
                    classification: None,
                };

                messages.push(msg);
            }
        }

        Ok(messages)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFilters {
    pub sender_id: Option<String>,
    pub keyword: Option<String>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub classification: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxStatistics {
    pub total_messages: u64,
    pub unread_messages: u64,
    pub total_conversations: u64,
    pub category_counts: HashMap<String, u64>,
    pub label_counts: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verify_webhook() {
        let collector = FacebookInboxCollector::new();

        let config = InboxConfig {
            page_id: "test_page".to_string(),
            access_token: "test_token".to_string(),
            verify_token: "my_secret_token".to_string(),
            webhook_secret: String::new(),
            auto_reply: false,
            routing_enabled: false,
        };

        collector.configure(config).await.unwrap();

        let result = collector
            .verify_webhook("subscribe", "my_secret_token", "test_challenge")
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_challenge");
    }

    #[tokio::test]
    async fn test_classification() {
        let collector = FacebookInboxCollector::new();

        let message = FacebookMessage {
            id: "test".to_string(),
            sender_id: "user1".to_string(),
            sender_name: "Test User".to_string(),
            recipient_id: "page1".to_string(),
            content: "Tôi muốn đặt hàng".to_string(),
            timestamp: Utc::now(),
            attachments: vec![],
            thread_id: "thread1".to_string(),
            is_read: false,
            labels: vec![],
            routed_to: None,
            classification: None,
        };

        let classified = collector.classify_message(&message).await;
        assert!(classified.classification.is_some());
        assert_eq!(classified.classification.unwrap().category, "order");
    }

    #[tokio::test]
    async fn test_statistics() {
        let collector = FacebookInboxCollector::new();

        let stats = collector.get_statistics().await;
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_conversations, 0);
    }
}
