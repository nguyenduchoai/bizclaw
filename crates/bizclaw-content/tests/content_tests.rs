#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use bizclaw_content::{
        ContentGenerator, ContentScheduler, LlmClient, TemplateManager,
        types::{
            Content, ContentCampaign, ContentLength, ContentMetrics, ContentPlatform,
            ContentStatus, ContentTemplate, ContentType, GenerationRequest, TemplateVariable, Tone,
        },
    };
    use std::collections::HashMap;
    use std::sync::Arc;

    struct MockLlmClient;

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn generate(&self, prompt: &str) -> anyhow::Result<String> {
            Ok(format!(
                "Generated content for: {}",
                &prompt[..prompt.len().min(50)]
            ))
        }
    }

    #[test]
    fn test_content_platform_display() {
        let platforms = vec![
            (ContentPlatform::Facebook, "facebook"),
            (ContentPlatform::Zalo, "zalo"),
            (ContentPlatform::TikTok, "tiktok"),
            (ContentPlatform::Shopee, "shopee"),
            (ContentPlatform::Website, "website"),
            (ContentPlatform::Email, "email"),
        ];

        for (platform, expected) in platforms {
            assert_eq!(platform.to_string(), expected);
        }
    }

    #[test]
    fn test_content_type_variants() {
        let types = vec![
            ContentType::Post,
            ContentType::Story,
            ContentType::Reel,
            ContentType::Ad,
            ContentType::Reply,
            ContentType::Comment,
            ContentType::ProductDescription,
            ContentType::Newsletter,
            ContentType::Announcement,
            ContentType::Carousel,
        ];

        assert_eq!(types.len(), 10);
    }

    #[test]
    fn test_tone_defaults() {
        let tone = Tone::default();
        assert_eq!(tone, Tone::Professional);

        let length = ContentLength::default();
        assert_eq!(length, ContentLength::Medium);
    }

    #[test]
    fn test_content_scheduler_add_content() {
        let mut scheduler = ContentScheduler::new();

        let content = Content {
            id: "content-1".to_string(),
            title: "Test Post".to_string(),
            body: "Test content body".to_string(),
            platform: ContentPlatform::Facebook,
            content_type: ContentType::Post,
            media: vec![],
            hashtags: vec!["#test".to_string()],
            created_at: chrono::Utc::now(),
            scheduled_at: None,
            status: ContentStatus::Draft,
        };

        let id = scheduler.add_content(content).unwrap();
        assert_eq!(id, "content-1");

        let retrieved = scheduler.get_content("content-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test Post");
    }

    #[test]
    fn test_content_scheduler_schedule() {
        let mut scheduler = ContentScheduler::new();

        let content = Content {
            id: "content-2".to_string(),
            title: "Scheduled Post".to_string(),
            body: "This will be scheduled".to_string(),
            platform: ContentPlatform::Zalo,
            content_type: ContentType::Post,
            media: vec![],
            hashtags: vec![],
            created_at: chrono::Utc::now(),
            scheduled_at: None,
            status: ContentStatus::Draft,
        };

        scheduler.add_content(content).unwrap();

        let future_time = chrono::Utc::now() + chrono::Duration::hours(2);
        scheduler
            .schedule_content("content-2", future_time)
            .unwrap();

        let content = scheduler.get_content("content-2").unwrap();
        assert_eq!(content.status, ContentStatus::Scheduled);
        assert!(content.scheduled_at.is_some());
    }

    #[test]
    fn test_content_scheduler_publish_now() {
        let mut scheduler = ContentScheduler::new();

        let content = Content {
            id: "content-3".to_string(),
            title: "Immediate Post".to_string(),
            body: "Publish immediately".to_string(),
            platform: ContentPlatform::TikTok,
            content_type: ContentType::Reel,
            media: vec![],
            hashtags: vec![],
            created_at: chrono::Utc::now(),
            scheduled_at: None,
            status: ContentStatus::Draft,
        };

        scheduler.add_content(content).unwrap();
        let published = scheduler.publish_now("content-3").unwrap();

        assert_eq!(published.status, ContentStatus::Published);
    }

    #[test]
    fn test_content_scheduler_list_filter() {
        let mut scheduler = ContentScheduler::new();

        for i in 1..=5 {
            let content = Content {
                id: format!("content-{}", i),
                title: format!("Post {}", i),
                body: "Body".to_string(),
                platform: if i <= 3 {
                    ContentPlatform::Facebook
                } else {
                    ContentPlatform::Zalo
                },
                content_type: ContentType::Post,
                media: vec![],
                hashtags: vec![],
                created_at: chrono::Utc::now(),
                scheduled_at: None,
                status: if i == 2 {
                    ContentStatus::Published
                } else {
                    ContentStatus::Draft
                },
            };
            scheduler.add_content(content).unwrap();
        }

        let fb_posts = scheduler.list_content(Some(&ContentPlatform::Facebook), None);
        assert_eq!(fb_posts.len(), 3);

        let published = scheduler.list_content(None, Some(&ContentStatus::Published));
        assert_eq!(published.len(), 1);
    }

    #[test]
    fn test_template_manager_creation() {
        let manager = TemplateManager::new();
        let templates = manager.list_templates();

        assert!(!templates.is_empty());
    }

    #[test]
    fn test_template_manager_render() {
        let manager = TemplateManager::new();
        let mut variables = HashMap::new();
        variables.insert("product_name".to_string(), "Test Product".to_string());
        variables.insert("price".to_string(), "299.000đ".to_string());
        variables.insert("description".to_string(), "Great product".to_string());

        let result = manager.render_template("fb-product-launch", &variables);
        assert!(result.is_ok());

        let rendered = result.unwrap();
        assert!(rendered.contains("Test Product"));
        assert!(rendered.contains("299.000đ"));
    }

    #[test]
    fn test_template_manager_filter_platform() {
        let manager = TemplateManager::new();

        let fb_templates = manager.list_templates_by_platform(&ContentPlatform::Facebook);
        assert!(!fb_templates.is_empty());

        for t in fb_templates {
            assert_eq!(t.platform, ContentPlatform::Facebook);
        }
    }

    #[test]
    fn test_template_manager_filter_category() {
        let manager = TemplateManager::new();

        let product_templates = manager.list_templates_by_category("product");
        assert!(!product_templates.is_empty());
    }

    #[test]
    fn test_content_metrics() {
        let metrics = ContentMetrics {
            content_id: "content-1".to_string(),
            views: 1000,
            likes: 100,
            comments: 25,
            shares: 10,
            clicks: 50,
            conversions: 5,
            reach: 500,
            engagement_rate: 0.135,
            updated_at: chrono::Utc::now(),
        };

        assert_eq!(metrics.views, 1000);
        assert!(metrics.engagement_rate > 0.1);
    }

    #[test]
    fn test_generation_request() {
        let request = GenerationRequest {
            content_type: ContentType::Post,
            platform: ContentPlatform::Facebook,
            topic: "Kinh doanh online".to_string(),
            tone: Tone::Casual,
            target_audience: Some("SME owners".to_string()),
            keywords: vec!["kinh doanh".to_string(), "online".to_string()],
            length: ContentLength::Medium,
            include_media_suggestion: true,
            custom_variables: HashMap::new(),
        };

        assert_eq!(request.tone, Tone::Casual);
        assert!(request.include_media_suggestion);
        assert_eq!(request.keywords.len(), 2);
    }

    #[tokio::test]
    async fn test_content_generator() {
        let generator = ContentGenerator::new(Arc::new(MockLlmClient));

        let request = GenerationRequest {
            content_type: ContentType::Post,
            platform: ContentPlatform::Facebook,
            topic: "Test Topic".to_string(),
            tone: Tone::Professional,
            target_audience: None,
            keywords: vec![],
            length: ContentLength::Short,
            include_media_suggestion: false,
            custom_variables: HashMap::new(),
        };

        let result = generator.generate_content(request).await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert_eq!(content.platform, ContentPlatform::Facebook);
        assert!(!content.body.is_empty());
    }

    #[test]
    fn test_campaign_creation() {
        let campaign = ContentCampaign {
            id: "campaign-1".to_string(),
            name: "Summer Sale Campaign".to_string(),
            description: "Promotional campaign for summer".to_string(),
            start_date: chrono::Utc::now(),
            end_date: chrono::Utc::now() + chrono::Duration::days(30),
            platforms: vec![ContentPlatform::Facebook, ContentPlatform::Zalo],
            content_ids: vec!["c1".to_string(), "c2".to_string()],
            status: bizclaw_content::types::CampaignStatus::Planning,
        };

        assert_eq!(campaign.platforms.len(), 2);
        assert_eq!(campaign.content_ids.len(), 2);
    }

    #[test]
    fn test_template_variable() {
        let var = TemplateVariable {
            name: "test_var".to_string(),
            description: "A test variable".to_string(),
            required: true,
            default_value: Some("default".to_string()),
        };

        assert!(var.required);
        assert!(var.default_value.is_some());
    }

    #[test]
    fn test_optimal_times_facebook() {
        let scheduler = ContentScheduler::new();
        let times = scheduler.get_optimal_times(&ContentPlatform::Facebook);

        assert!(!times.is_empty());
        assert!(times.len() <= 14);
    }

    #[test]
    fn test_optimal_times_tiktok() {
        let scheduler = ContentScheduler::new();
        let times = scheduler.get_optimal_times(&ContentPlatform::TikTok);

        assert!(!times.is_empty());
    }
}
