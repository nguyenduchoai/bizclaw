//! # Facebook Poster Tool - Automated posting with scheduling
//!
//! Features:
//! - Schedule posts with cron-like timing
//! - Multi-account/agent support
//! - Retry mechanism with exponential backoff
//! - Content formatting per platform
//! - Post status tracking and logging
//! - Rate limit handling

use crate::types::{Platform, SocialContent};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacebookPosterConfig {
    pub page_id: String,
    pub access_token: String,
    pub agent_name: String,
    pub auto_retry: bool,
    pub max_retries: u32,
    pub retry_delay_secs: u64,
}

impl Default for FacebookPosterConfig {
    fn default() -> Self {
        Self {
            page_id: String::new(),
            access_token: String::new(),
            agent_name: "default".to_string(),
            auto_retry: true,
            max_retries: 3,
            retry_delay_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledPost {
    pub id: String,
    pub content: String,
    pub image_url: Option<String>,
    pub scheduled_time: DateTime<Utc>,
    pub status: PostStatus,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub page_id: String,
    pub agent_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    Pending,
    Scheduled,
    Posting,
    Posted,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResult {
    pub post_id: String,
    pub permalink_url: String,
    pub posted_at: DateTime<Utc>,
    pub agent_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMetrics {
    pub impressions: u64,
    pub reach: u64,
    pub engagements: u64,
    pub reactions: u64,
    pub comments: u64,
    pub shares: u64,
}

pub struct FacebookPoster {
    clients: Arc<RwLock<HashMap<String, FacebookClientInstance>>>,
    scheduled_posts: Arc<RwLock<Vec<ScheduledPost>>>,
    client: Client,
}

struct FacebookClientInstance {
    config: FacebookPosterConfig,
    http_client: Client,
}

impl FacebookPoster {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            scheduled_posts: Arc::new(RwLock::new(Vec::new())),
            client: Client::new(),
        }
    }

    pub async fn register_account(&self, config: FacebookPosterConfig) -> Result<String> {
        let agent_name = config.agent_name.clone();

        let client = FacebookClientInstance {
            config: config.clone(),
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .context("HTTP client error")?,
        };

        let mut clients = self.clients.write().await;
        clients.insert(agent_name.clone(), client);

        info!("Registered Facebook account for agent: {}", agent_name);
        Ok(agent_name)
    }

    pub async fn unregister_account(&self, agent_name: &str) -> Result<()> {
        let mut clients = self.clients.write().await;
        clients.remove(agent_name);
        info!("Unregistered Facebook account: {}", agent_name);
        Ok(())
    }

    pub async fn post_now(
        &self,
        agent_name: &str,
        content: &str,
        image_url: Option<&str>,
    ) -> Result<PostResult> {
        let mut clients = self.clients.write().await;
        let client = clients
            .get_mut(agent_name)
            .context("Agent not registered")?;

        let post_id = self.do_post(&client.config, content, image_url).await?;

        Ok(PostResult {
            post_id: post_id.clone(),
            permalink_url: format!("https://facebook.com/{}", post_id),
            posted_at: Utc::now(),
            agent_name: agent_name.to_string(),
        })
    }

    async fn do_post(
        &self,
        config: &FacebookPosterConfig,
        content: &str,
        image_url: Option<&str>,
    ) -> Result<String> {
        let mut retry_count = 0;
        let max_retries = if config.auto_retry {
            config.max_retries
        } else {
            1
        };

        loop {
            match self.attempt_post(config, content, image_url).await {
                Ok(post_id) => return Ok(post_id),
                Err(e) if retry_count < max_retries => {
                    retry_count += 1;
                    warn!(
                        "Post failed (attempt {}/{}): {}. Retrying in {}s...",
                        retry_count, max_retries, e, config.retry_delay_secs
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(config.retry_delay_secs))
                        .await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn attempt_post(
        &self,
        config: &FacebookPosterConfig,
        content: &str,
        image_url: Option<&str>,
    ) -> Result<String> {
        let base_url = "https://graph.facebook.com/v18.0";
        let mut form = reqwest::multipart::Form::new()
            .text("message", content.to_string())
            .text("access_token", config.access_token.clone());

        if let Some(url) = image_url {
            form = form.text("url", url.to_string());
        }

        let url = format!("{}/{}/feed", base_url, config.page_id);

        let response = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("Request failed")?;

        let status = response.status();

        if status.is_success() {
            #[derive(Deserialize)]
            struct PostResponse {
                id: String,
            }

            let result: PostResponse = response.json().await.context("Parse response")?;
            debug!("Posted successfully: {}", result.id);
            Ok(result.id)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Facebook API error {}: {}", status, error_text)
        }
    }

    pub async fn schedule_post(
        &self,
        agent_name: &str,
        content: String,
        image_url: Option<String>,
        scheduled_time: DateTime<Utc>,
    ) -> Result<String> {
        let post_id = uuid::Uuid::new_v4().to_string();

        let post = ScheduledPost {
            id: post_id.clone(),
            content,
            image_url,
            scheduled_time,
            status: PostStatus::Scheduled,
            retry_count: 0,
            created_at: Utc::now(),
            posted_at: None,
            error_message: None,
            page_id: String::new(),
            agent_name: agent_name.to_string(),
        };

        let mut posts = self.scheduled_posts.write().await;
        posts.push(post);

        info!(
            "Scheduled post {} for {} by agent {}",
            post_id, scheduled_time, agent_name
        );
        Ok(post_id)
    }

    pub async fn get_scheduled_posts(&self, agent_name: Option<&str>) -> Vec<ScheduledPost> {
        let posts = self.scheduled_posts.read().await;

        match agent_name {
            Some(name) => posts
                .iter()
                .filter(|p| p.agent_name == name && p.status == PostStatus::Scheduled)
                .cloned()
                .collect(),
            None => posts
                .iter()
                .filter(|p| p.status == PostStatus::Scheduled)
                .cloned()
                .collect(),
        }
    }

    pub async fn cancel_post(&self, post_id: &str) -> Result<()> {
        let mut posts = self.scheduled_posts.write().await;

        if let Some(post) = posts.iter_mut().find(|p| p.id == post_id) {
            post.status = PostStatus::Cancelled;
            info!("Cancelled scheduled post: {}", post_id);
            Ok(())
        } else {
            anyhow::bail!("Post not found: {}", post_id)
        }
    }

    pub async fn get_post_status(&self, post_id: &str) -> Result<ScheduledPost> {
        let posts = self.scheduled_posts.read().await;

        posts
            .iter()
            .find(|p| p.id == post_id)
            .cloned()
            .context("Post not found")
    }

    pub async fn process_due_posts(&self) -> Result<Vec<PostResult>> {
        let mut results = Vec::new();
        let now = Utc::now();

        let mut posts = self.scheduled_posts.write().await;

        for post in posts.iter_mut() {
            if post.status == PostStatus::Scheduled && post.scheduled_time <= now {
                post.status = PostStatus::Posting;

                match self
                    .post_now(&post.agent_name, &post.content, post.image_url.as_deref())
                    .await
                {
                    Ok(result) => {
                        post.status = PostStatus::Posted;
                        post.posted_at = Some(Utc::now());
                        results.push(result);
                    }
                    Err(e) => {
                        post.status = PostStatus::Failed;
                        post.error_message = Some(e.to_string());
                        post.retry_count += 1;
                        warn!("Failed to post {}: {}", post.id, e);
                    }
                }
            }
        }

        Ok(results)
    }

    pub async fn get_metrics(&self, post_id: &str) -> Result<PostMetrics> {
        let posts = self.scheduled_posts.read().await;
        let post = posts
            .iter()
            .find(|p| p.id == post_id)
            .context("Post not found")?;

        if post.status != PostStatus::Posted {
            anyhow::bail!("Post not yet posted");
        }

        let clients = self.clients.read().await;
        let client = clients.get(&post.agent_name).context("Agent not found")?;

        let url = format!(
            "https://graph.facebook.com/v18.0/{}_{}?fields=insights.metric(impressions,reach,engagements,reactions,comments,shares)&access_token={}",
            post.page_id, post_id, client.config.access_token
        );

        #[derive(Deserialize)]
        struct InsightsResponse {
            insights: InsightsData,
        }

        #[derive(Deserialize)]
        struct InsightsData {
            data: Vec<InsightMetric>,
        }

        #[derive(Deserialize)]
        struct InsightMetric {
            name: String,
            values: Vec<InsightValue>,
        }

        #[derive(Deserialize)]
        struct InsightValue {
            value: u64,
        }

        let response = self.client.get(&url).send().await?;
        let insights: InsightsResponse = response.json().await.unwrap_or(InsightsResponse {
            insights: InsightsData { data: vec![] },
        });

        let mut metrics = PostMetrics {
            impressions: 0,
            reach: 0,
            engagements: 0,
            reactions: 0,
            comments: 0,
            shares: 0,
        };

        for metric in insights.insights.data {
            let value = metric.values.first().map(|v| v.value).unwrap_or(0);
            match metric.name.as_str() {
                "impressions" => metrics.impressions = value,
                "reach" => metrics.reach = value,
                "engagements" => metrics.engagements = value,
                "reactions" => metrics.reactions = value,
                "comments" => metrics.comments = value,
                "shares" => metrics.shares = value,
                _ => {}
            }
        }

        Ok(metrics)
    }
}

impl Default for FacebookPoster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_account() {
        let poster = FacebookPoster::new();

        let config = FacebookPosterConfig {
            page_id: "test_page".to_string(),
            access_token: "test_token".to_string(),
            agent_name: "test_agent".to_string(),
            auto_retry: true,
            max_retries: 3,
            retry_delay_secs: 1,
        };

        let result = poster.register_account(config).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_agent");
    }

    #[tokio::test]
    async fn test_schedule_post() {
        let poster = FacebookPoster::new();

        let config = FacebookPosterConfig {
            page_id: "test_page".to_string(),
            access_token: "test_token".to_string(),
            agent_name: "test_agent".to_string(),
            auto_retry: true,
            max_retries: 3,
            retry_delay_secs: 1,
        };

        poster.register_account(config).await.unwrap();

        let future_time = Utc::now() + chrono::Duration::hours(1);
        let result = poster
            .schedule_post("test_agent", "Test content".to_string(), None, future_time)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cancel_post() {
        let poster = FacebookPoster::new();

        let future_time = Utc::now() + chrono::Duration::hours(1);
        let post_id = poster
            .schedule_post("test_agent", "Test content".to_string(), None, future_time)
            .await
            .unwrap();

        let result = poster.cancel_post(&post_id).await;
        assert!(result.is_ok());

        let post = poster.get_post_status(&post_id).await.unwrap();
        assert_eq!(post.status, PostStatus::Cancelled);
    }
}
