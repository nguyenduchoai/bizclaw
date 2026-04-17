//! # BizClaw CRM - AI Social CRM / OmniCRM
//!
//! Unified inbox + CRM cho SME với AI-powered features
//!
//! ## Features:
//! - Multi-platform inbox (Zalo, Facebook, Instagram, etc.)
//! - Customer Pipeline (New → Contacted → Interested → Converted → Lost)
//! - AI Reply Assistant
//! - Template Messages with variables
//! - Workflow Automation
//! - Dashboard & Analytics
//! - **OmniChannel Customer 360** - Tất cả kênh hợp nhất
//! - **Contact Deduplication** - Gộp trùng bằng fuzzy matching
//! - **Real-time Sync** - Cập nhật từ mọi kênh

pub mod manager;
pub mod omni;
pub mod types;

pub use manager::CRMManager;
pub use omni::{
    ActivitySummary, Channel, ChannelAccount, ChannelContact, ChannelSummary, Customer360,
    Customer360Manager, CustomerPreferences, DedupeEngine, DedupeMatch, DedupeThresholds,
    Direction, Interaction, InteractionType, LifetimeValue, MatchType, NBAType, NextBestAction,
    Review, Sentiment, SupportTicket, Transaction,
};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_contact() {
        let crm = CRMManager::new();

        let contact = Contact {
            id: uuid::Uuid::new_v4().to_string(),
            crm_name: Some("Nguyễn Văn A".to_string()),
            platform_name: "Nguyễn Văn A".to_string(),
            platform: Platform::Zalo,
            platform_id: "zalo_123".to_string(),
            phone: Some("0901234567".to_string()),
            email: None,
            avatar: None,
            pipeline_status: PipelineStatus::New,
            tags: vec!["vip".to_string()],
            notes: String::new(),
            custom_fields: std::collections::HashMap::new(),
            source: ContactSource::Organic,
            assigned_to: None,
            score: 80,
            total_spent: 0.0,
            message_count: 0,
            last_message_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let id = crm.create_contact(contact).await.unwrap();
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_update() {
        let crm = CRMManager::new();

        let contact = Contact {
            id: "test_1".to_string(),
            crm_name: Some("Test".to_string()),
            platform_name: "Test".to_string(),
            platform: Platform::Facebook,
            platform_id: "fb_123".to_string(),
            phone: None,
            email: None,
            avatar: None,
            pipeline_status: PipelineStatus::New,
            tags: vec![],
            notes: String::new(),
            custom_fields: std::collections::HashMap::new(),
            source: ContactSource::Organic,
            assigned_to: None,
            score: 0,
            total_spent: 0.0,
            message_count: 0,
            last_message_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        crm.create_contact(contact).await.unwrap();
        crm.update_pipeline("test_1", PipelineStatus::Contacted)
            .await
            .unwrap();

        let updated = crm.get_contact("test_1").await.unwrap();
        assert_eq!(updated.pipeline_status, PipelineStatus::Contacted);
    }

    #[tokio::test]
    async fn test_dashboard() {
        let crm = CRMManager::new();
        let dashboard = crm.get_dashboard().await;

        assert_eq!(dashboard.total_contacts, 0);
        assert_eq!(dashboard.unread_messages, 0);
    }

    #[tokio::test]
    async fn test_dedupe_engine() {
        use std::collections::HashMap;

        let engine = DedupeEngine::new();

        let contact1 = ChannelContact {
            channel: Channel::Zalo,
            channel_contact_id: "z1".to_string(),
            display_name: "Nguyễn Văn A".to_string(),
            avatar: None,
            phone: vec!["0901234567".to_string()],
            email: vec![],
            address: None,
            dob: None,
            gender: None,
            is_verified: false,
            metadata: HashMap::new(),
            linked_at: chrono::Utc::now(),
        };

        let contact2 = ChannelContact {
            channel: Channel::Facebook,
            channel_contact_id: "fb1".to_string(),
            display_name: "Nguyễn Văn A".to_string(),
            avatar: None,
            phone: vec!["0901234567".to_string()],
            email: vec!["nguyenvana@email.com".to_string()],
            address: None,
            dob: None,
            gender: None,
            is_verified: false,
            metadata: HashMap::new(),
            linked_at: chrono::Utc::now(),
        };

        let merged = engine.merge_contacts(vec![contact1, contact2]);

        assert_eq!(merged.channels.len(), 2);
        assert!(merged.merge_confidence > 0.0);
    }
}
