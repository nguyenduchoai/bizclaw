//! # Sources Module
//!
//! Multi-source news aggregation for TrendRadar.

use async_trait::async_trait;
use crate::{NewsItem, SourceType};

fn now_rfc3339() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let _ts = now; // UTC offset
    format!("{}Z", now)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceConfig {
    pub source_type: SourceType,
    pub url: String,
    pub name: String,
    pub enabled: bool,
}

#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn source_type(&self) -> SourceType;
    async fn fetch(&mut self) -> anyhow::Result<Vec<NewsItem>>;
}

pub fn create_source(config: &SourceConfig) -> Box<dyn Source> {
    match config.source_type {
        SourceType::Rss => Box::new(RssSource::new(&config.url, &config.name)),
        SourceType::Api => Box::new(ApiSource::new(&config.url, &config.name)),
        SourceType::Webhook => Box::new(WebhookSource::new(&config.url, &config.name)),
    }
}

#[derive(Debug, Clone)]
pub struct RssSource {
    pub url: String,
    pub name: String,
}

impl RssSource {
    pub fn new(url: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            name: name.into(),
        }
    }

    async fn parse_feed(&self, content: &str) -> anyhow::Result<Vec<NewsItem>> {
        use feed_rs::parser;

        let feed = parser::parse(content.as_bytes())?;
        let mut items = Vec::new();

        for (i, entry) in feed.entries.iter().enumerate() {
            let title = entry
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_default();

            let url = entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default();

            let summary = entry.summary.as_ref().map(|s| s.content.clone());

            let published_at = entry
                .published
                .or(entry.updated)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(now_rfc3339);

            items.push(NewsItem {
                id: format!("{}_{}", self.name.to_lowercase().replace(' ', "_"), i),
                title,
                url,
                summary,
                source: self.name.clone(),
                source_type: SourceType::Rss,
                published_at,
                rank: None,
                mentions: 100,
                hotness: 0.5,
                tags: Vec::new(),
            });
        }

        Ok(items)
    }
}

#[async_trait]
impl Source for RssSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> SourceType {
        SourceType::Rss
    }

    async fn fetch(&mut self) -> anyhow::Result<Vec<NewsItem>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client
            .get(&self.url)
            .header("User-Agent", "BizClaw-TrendRadar/1.0")
            .header("Accept", "application/rss+xml, application/xml, text/xml, */*")
            .send()
            .await?;

        let content = response.text().await?;
        self.parse_feed(&content).await
    }
}

#[derive(Debug, Clone)]
pub struct ApiSource {
    pub url: String,
    pub name: String,
}

impl ApiSource {
    pub fn new(url: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            name: name.into(),
        }
    }
}

#[async_trait]
impl Source for ApiSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> SourceType {
        SourceType::Api
    }

    async fn fetch(&mut self) -> anyhow::Result<Vec<NewsItem>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client
            .get(&self.url)
            .header("User-Agent", "BizClaw-TrendRadar/1.0")
            .header("Accept", "application/json")
            .send()
            .await?;

        let text = response.text().await?;

        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
            return Ok(items
                .into_iter()
                .enumerate()
                .map(|(i, v)| NewsItem {
                    id: format!("{}_{}", self.name.to_lowercase().replace(' ', "_"), i),
                    title: v.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                    url: v.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                    summary: v.get("summary").and_then(|s| s.as_str()).map(|s| s.to_string()),
                    source: self.name.clone(),
                    source_type: SourceType::Api,
                    published_at: v.get("published_at")
                        .and_then(|p| p.as_str())
                        .unwrap_or(&now_rfc3339())
                        .to_string(),
                    rank: v.get("rank").and_then(|r| r.as_u64()).map(|r| r as u32),
                    mentions: v.get("mentions").and_then(|m| m.as_u64()).unwrap_or(100),
                    hotness: v.get("hotness").and_then(|h| h.as_f64()).unwrap_or(0.5) as f32,
                    tags: Vec::new(),
                })
                .collect());
        }

        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct WebhookSource {
    pub url: String,
    pub name: String,
    buffer: Vec<NewsItem>,
}

impl WebhookSource {
    pub fn new(url: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            name: name.into(),
            buffer: Vec::new(),
        }
    }

    pub fn push(&mut self, item: NewsItem) {
        self.buffer.push(item);
    }

    pub fn flush(&mut self) -> Vec<NewsItem> {
        let items: Vec<NewsItem> = self.buffer.drain(..).collect();
        items
    }
}

#[async_trait]
impl Source for WebhookSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> SourceType {
        SourceType::Webhook
    }

    async fn fetch(&mut self) -> anyhow::Result<Vec<NewsItem>> {
        Ok(self.flush())
    }
}
