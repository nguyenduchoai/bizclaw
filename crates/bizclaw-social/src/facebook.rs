use crate::types::*;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct FacebookClient {
    access_token: Arc<RwLock<Option<String>>>,
    page_id: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPageInfo {
    pub id: String,
    pub name: String,
    pub about: Option<String>,
    pub fan_count: u64,
    pub followers_count: u64,
    pub picture: FacebookPicture,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPicture {
    pub data: FacebookPictureData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPictureData {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookPost {
    pub id: String,
    pub message: Option<String>,
    pub created_time: String,
    pub full_picture: Option<String>,
    pub permalink_url: String,
    pub shares: Option<FacebookShares>,
    pub reactions: Option<FacebookReactions>,
    pub comments: Option<FacebookComments>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookShares {
    pub count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookReactions {
    pub data: Vec<FacebookReaction>,
    pub summary: Option<FacebookSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookReaction {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub reaction_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookComments {
    pub data: Vec<FacebookComment>,
    pub summary: Option<FacebookSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookComment {
    pub id: String,
    pub message: String,
    pub created_time: String,
    pub from: FacebookUser,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookUser {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookSummary {
    pub total_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookInsights {
    pub data: Vec<FacebookInsightData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookInsightData {
    pub name: String,
    pub period: String,
    pub values: Vec<FacebookInsightValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FacebookInsightValue {
    pub value: f64,
    pub end_time: String,
}

impl FacebookClient {
    pub fn new(access_token: String, page_id: Option<String>) -> Self {
        Self {
            access_token: Arc::new(RwLock::new(Some(access_token))),
            page_id: Arc::new(RwLock::new(page_id)),
            client: Client::new(),
            base_url: "https://graph.facebook.com/v18.0".to_string(),
        }
    }

    pub async fn set_access_token(&self, token: String) {
        let mut guard = self.access_token.write().await;
        *guard = Some(token);
    }

    pub async fn set_page_id(&self, page_id: String) {
        let mut guard = self.page_id.write().await;
        *guard = Some(page_id);
    }

    pub async fn is_authenticated(&self) -> bool {
        let guard = self.access_token.read().await;
        guard.is_some() && guard.as_ref().unwrap().len() > 10
    }

    pub async fn get_page_info(&self) -> Result<FacebookPageInfo> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let page_id = self.page_id.read().await;
        let page_id = page_id.as_ref().context("No page ID")?;

        let url = format!(
            "{}/{}?fields=id,name,about,fan_count,followers_count,picture&access_token={}",
            self.base_url, page_id, token
        );

        let response = self.client.get(&url).send().await?;
        let info: FacebookPageInfo = response.json().await?;
        Ok(info)
    }

    pub async fn create_post(&self, message: &str, image_url: Option<&str>) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let page_id = self.page_id.read().await;
        let page_id = page_id.as_ref().context("No page ID")?;

        let mut form = reqwest::multipart::Form::new()
            .text("message", message.to_string())
            .text("access_token", token.clone());

        if let Some(url) = image_url {
            form = form.text("url", url.to_string());
        }

        let url = format!("{}/{}/feed", self.base_url, page_id);

        let response = self.client.post(&url).multipart(form).send().await?;

        #[derive(Deserialize)]
        struct PostResponse {
            id: String,
        }

        let result: PostResponse = response.json().await?;
        Ok(result.id)
    }

    pub async fn upload_photo(&self, photo_url: &str, caption: &str) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let page_id = self.page_id.read().await;
        let page_id = page_id.as_ref().context("No page ID")?;

        let params = [
            ("url", photo_url),
            ("caption", caption),
            ("access_token", token),
        ];

        let url = format!("{}/{}/photos", self.base_url, page_id);

        let response = self.client.post(&url).form(&params).send().await?;

        #[derive(Deserialize)]
        struct PhotoResponse {
            id: String,
            post_id: Option<String>,
        }

        let result: PhotoResponse = response.json().await?;
        Ok(result.id)
    }

    pub async fn get_posts(&self, limit: u32) -> Result<Vec<FacebookPost>> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let page_id = self.page_id.read().await;
        let page_id = page_id.as_ref().context("No page ID")?;

        let url = format!(
            "{}/{}/posts?fields=id,message,created_time,full_picture,permalink_url,shares,reactions.summary(true),comments.summary(true)&limit={}&access_token={}",
            self.base_url, page_id, limit, token
        );

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct PostsResponse {
            data: Vec<FacebookPost>,
        }

        let result: PostsResponse = response.json().await?;
        Ok(result.data)
    }

    pub async fn get_insights(
        &self,
        metric: &str,
        period: &str,
    ) -> Result<Vec<FacebookInsightData>> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let page_id = self.page_id.read().await;
        let page_id = page_id.as_ref().context("No page ID")?;

        let url = format!(
            "{}/{}/insights?metric={}&period={}&access_token={}",
            self.base_url, page_id, metric, period, token
        );

        let response = self.client.get(&url).send().await?;
        let insights: FacebookInsights = response.json().await?;
        Ok(insights.data)
    }

    pub async fn delete_post(&self, post_id: &str) -> Result<()> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let url = format!("{}/{}?access_token={}", self.base_url, post_id, token);

        let response = self.client.delete(&url).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to delete post")
        }
    }

    pub fn generate_auth_url(&self, app_id: &str, redirect_uri: &str, state: &str) -> String {
        format!(
            "https://www.facebook.com/v18.0/dialog/oauth?client_id={}&redirect_uri={}&state={}&scope=pages_read_engagement,pages_manage_posts,instagram_basic,instagram_content_publish",
            app_id, redirect_uri, state
        )
    }

    pub async fn exchange_token(&self, short_lived_token: &str) -> Result<String> {
        let url = format!(
            "{}/oauth/access_token?grant_type=fb_exchange_token&client_id={}&client_secret={}&fb_exchange_token={}",
            self.base_url, self.access_token.read().await.as_ref().unwrap_or(&String::new()), "", short_lived_token
        );

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: u64,
        }

        let response = self.client.get(&url).send().await?;
        let result: TokenResponse = response.json().await?;
        Ok(result.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_facebook_client_authentication() {
        let client = FacebookClient::new(
            "test_token_12345678901234567890".to_string(),
            Some("test_page_123".to_string()),
        );
        assert!(client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_facebook_client_not_authenticated() {
        let client = FacebookClient::new("short".to_string(), None);
        assert!(!client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_set_credentials() {
        let client = FacebookClient::new("initial_token".to_string(), Some("page_1".to_string()));

        client.set_access_token("new_token".to_string()).await;
        client.set_page_id("page_2".to_string()).await;

        assert!(client.is_authenticated().await);

        let page_id = client.page_id.read().await;
        assert_eq!(*page_id, Some("page_2".to_string()));
    }

    #[test]
    fn test_generate_auth_url() {
        let client = FacebookClient::new("test_token".to_string(), None);

        let url = client.generate_auth_url("my_app_id", "https://myapp.com/callback", "my_state");

        assert!(url.contains("client_id=my_app_id"));
        assert!(url.contains("redirect_uri=https://myapp.com/callback"));
    }
}
