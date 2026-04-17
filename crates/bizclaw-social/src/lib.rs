#![doc = include_str!("../README.md")]

pub mod adapters;
pub mod facebook;
pub mod facebook_inbox;
pub mod facebook_poster;
pub mod instagram;
pub mod scheduler;
pub mod social_manager;
pub mod tiktok;
pub mod types;
pub mod zalo;

pub use adapters::SocialAdapter;
pub use facebook::FacebookClient;
pub use facebook_inbox::{
    Conversation, FacebookInboxCollector, FacebookMessage, InboxConfig, InboxStatistics,
    MessageClassification, MessageFilters,
};
pub use facebook_poster::{
    FacebookPoster, FacebookPosterConfig, PostMetrics, PostResult, PostStatus, ScheduledPost,
};
pub use instagram::InstagramClient;
pub use social_manager::{
    Comment, DirectMessage, Insights, PlatformCredentials, PostContent, PostResponse,
    SocialMediaManager,
};
pub use tiktok::TikTokClient;
pub use types::*;
pub use zalo::ZaloClient;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_enum() {
        assert_eq!(Platform::ZaloOA.code(), "zalo");
        assert_eq!(Platform::TikTok.code(), "tiktok");
        assert_eq!(Platform::Facebook.code(), "facebook");
    }

    #[test]
    fn test_post_status() {
        assert_eq!(PostStatus::Pending, PostStatus::Pending);
        assert_eq!(PostStatus::Scheduled, PostStatus::Scheduled);
        assert_eq!(PostStatus::Posted, PostStatus::Posted);
    }

    #[test]
    fn test_social_content_builder() {
        let content = SocialContent::builder()
            .text("Test post content")
            .add_hashtag("bizclaw")
            .add_hashtag("ai")
            .platform(Platform::ZaloOA)
            .build();

        assert_eq!(content.text, "Test post content");
        assert_eq!(content.hashtags, vec!["bizclaw", "ai"]);
        assert_eq!(content.platform, Platform::ZaloOA);
    }

    #[tokio::test]
    async fn test_zalo_client_creation() {
        let client = ZaloClient::new("test_access_token".to_string());
        assert!(client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_tiktok_client_creation() {
        let client = TikTokClient::new(
            "test_client_key".to_string(),
            "test_client_secret".to_string(),
        );
        assert!(!client.is_authenticated().await);
    }
}
