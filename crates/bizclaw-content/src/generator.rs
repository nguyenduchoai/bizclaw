use crate::types::{
    Content, ContentLength, ContentPlatform, ContentStatus, ContentType, GenerationRequest, Media,
    MediaType, Tone,
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

pub struct ContentGenerator {
    llm_client: Arc<dyn LlmClient>,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String>;
}

impl ContentGenerator {
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self { llm_client }
    }

    pub async fn generate_content(&self, request: GenerationRequest) -> Result<Content> {
        let prompt = self.build_prompt(&request);
        let generated_text = self.llm_client.generate(&prompt).await?;

        let content = self.parse_generated_content(&generated_text, &request)?;

        Ok(content)
    }

    fn build_prompt(&self, request: &GenerationRequest) -> String {
        let length_instruction = match request.length {
            ContentLength::Short => "Keep it concise, under 150 characters.",
            ContentLength::Medium => "Write a balanced content, around 200-500 characters.",
            ContentLength::Long => "Write detailed content, around 500-1500 characters.",
        };

        let tone_instruction = match request.tone {
            Tone::Professional => "Use professional and formal language.",
            Tone::Casual => "Use friendly and casual language, like talking to a friend.",
            Tone::Humorous => "Add humor and wit to engage readers.",
            Tone::Inspirational => "Inspire and motivate the audience.",
            Tone::Urgent => "Create urgency and emphasize time-sensitivity.",
            Tone::Educational => "Inform and educate the audience about the topic.",
            Tone::Promotional => "Highlight benefits and include call-to-action.",
        };

        let audience_instruction = request
            .target_audience
            .as_ref()
            .map(|a| format!("Target audience: {}.", a))
            .unwrap_or_default();

        let keywords_instruction = if request.keywords.is_empty() {
            String::new()
        } else {
            format!(
                "Include these keywords naturally: {}.",
                request.keywords.join(", ")
            )
        };

        let platform_instruction = match request.platform {
            ContentPlatform::Facebook => {
                "Optimized for Facebook feed. Can include emojis and hashtags."
            }
            ContentPlatform::Zalo => "Suitable for Zalo OA. Use Vietnamese naturally.",
            ContentPlatform::TikTok => {
                "Catchy and trendy. Short, punchy sentences. Include trending elements."
            }
            ContentPlatform::Shopee => {
                "E-commerce focused. Highlight product benefits and include price info."
            }
            ContentPlatform::Website => {
                "SEO-optimized blog post. Include proper headings structure."
            }
            ContentPlatform::Email => "Email marketing format. Strong subject line, clear body.",
        };

        let media_instruction = if request.include_media_suggestion {
            "Suggest appropriate media type (image/video/carousel) at the end."
        } else {
            "Focus on text content only."
        };

        format!(
            r#"Generate a {} for {}.

Content Type: {:?}

Topic: {}

{}
{}
{}
{}
{}

{}
"#,
            request.platform,
            request.content_type.debug_fallback(),
            request.content_type,
            request.topic,
            length_instruction,
            tone_instruction,
            audience_instruction,
            keywords_instruction,
            platform_instruction,
            media_instruction
        )
    }

    fn parse_generated_content(
        &self,
        generated: &str,
        request: &GenerationRequest,
    ) -> Result<Content> {
        let (title, body) = if generated.contains('\n') {
            let mut parts = generated.trim().splitn(2, '\n');
            let title = parts.next().unwrap_or("").trim().to_string();
            let body = parts.next().unwrap_or(generated).trim().to_string();
            (title, body)
        } else {
            ("".to_string(), generated.trim().to_string())
        };

        let hashtags = self.extract_hashtags(&body);
        let media = if request.include_media_suggestion {
            self.suggest_media(&body, request.content_type.clone())
        } else {
            vec![]
        };

        let content_type = match request.content_type {
            ContentType::ProductDescription => ContentType::ProductDescription,
            ContentType::Newsletter => ContentType::Newsletter,
            _ => request.content_type.clone(),
        };

        Ok(Content {
            id: uuid_v4(),
            title,
            body,
            platform: request.platform.clone(),
            content_type,
            media,
            hashtags,
            created_at: Utc::now(),
            scheduled_at: None,
            status: ContentStatus::Draft,
        })
    }

    fn extract_hashtags(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.starts_with('#'))
            .map(|s| {
                s.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                    .to_string()
            })
            .collect()
    }

    fn suggest_media(&self, body: &str, content_type: ContentType) -> Vec<Media> {
        let media_type = match content_type {
            ContentType::Reel | ContentType::Story => MediaType::Video,
            ContentType::Carousel => MediaType::Carousel,
            _ => MediaType::Image,
        };

        vec![Media {
            media_type,
            url: String::new(),
            caption: Some(format!(
                "Suggested media for: {}",
                &body[..body.len().min(100)]
            )),
        }]
    }
}

impl ContentType {
    fn debug_fallback(&self) -> &'static str {
        match self {
            ContentType::Post => "Post",
            ContentType::Story => "Story",
            ContentType::Reel => "Reel",
            ContentType::Ad => "Advertisement",
            ContentType::Reply => "Reply",
            ContentType::Comment => "Comment",
            ContentType::ProductDescription => "Product Description",
            ContentType::Newsletter => "Newsletter",
            ContentType::Announcement => "Announcement",
            ContentType::Carousel => "Carousel",
        }
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{:x}-{:x}-4{:x}-{:x}-{:x}",
        timestamp >> 96,
        (timestamp >> 64) & 0xFFFF,
        (timestamp >> 48) & 0xFFF,
        0x8000 | ((timestamp >> 32) & 0x3FFF),
        timestamp & 0xFFFFFFFFFFFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLlmClient;

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn generate(&self, _prompt: &str) -> Result<String> {
            Ok(
                "Tiêu đề bài viết\nNội dung bài viết mẫu với hashtag #vietnamese #business"
                    .to_string(),
            )
        }
    }

    #[tokio::test]
    async fn test_content_generation() {
        let generator = ContentGenerator::new(Arc::new(MockLlmClient));

        let request = GenerationRequest {
            content_type: ContentType::Post,
            platform: ContentPlatform::Facebook,
            topic: "Kinh doanh online".to_string(),
            tone: Tone::Professional,
            target_audience: Some("SME owners".to_string()),
            keywords: vec!["kinh doanh".to_string(), "online".to_string()],
            length: ContentLength::Medium,
            include_media_suggestion: false,
            custom_variables: std::collections::HashMap::new(),
        };

        let result = generator.generate_content(request).await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert_eq!(content.platform, ContentPlatform::Facebook);
        assert!(!content.body.is_empty());
    }

    #[test]
    fn test_hashtag_extraction() {
        let generator = ContentGenerator::new(Arc::new(MockLlmClient));
        let text = "Check out #Vietnam #Business and #SME trends!";
        let hashtags = generator.extract_hashtags(text);

        assert_eq!(hashtags.len(), 3);
        assert!(hashtags.contains(&"Vietnam".to_string()));
        assert!(hashtags.contains(&"Business".to_string()));
        assert!(hashtags.contains(&"SME".to_string()));
    }
}
