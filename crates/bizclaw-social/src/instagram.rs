use crate::types::*;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone)]
pub struct InstagramClient {
    access_token: Arc<RwLock<Option<String>>>,
    ig_user_id: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstagramBusinessAccount {
    pub id: String,
    pub instagram_business_account: InstagramAccountId,
    pub name: String,
    pub followers_count: u64,
    pub follows_count: u64,
    pub media_count: u64,
    pub profile_picture_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstagramAccountId {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstagramMedia {
    pub id: String,
    pub caption: Option<String>,
    pub media_type: String,
    pub media_url: String,
    pub permalink: String,
    pub timestamp: String,
    pub username: String,
    pub children: Option<InstagramChildren>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstagramChildren {
    pub data: Vec<InstagramMediaItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstagramMediaItem {
    pub id: String,
    pub media_type: String,
    pub media_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstagramMediaResponse {
    pub id: String,
    pub caption: String,
    pub permalink: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstagramCarouselItem {
    pub image_url: String,
    pub caption: Option<String>,
}

impl InstagramClient {
    pub fn new(access_token: String, ig_user_id: Option<String>) -> Self {
        Self {
            access_token: Arc::new(RwLock::new(Some(access_token))),
            ig_user_id: Arc::new(RwLock::new(ig_user_id)),
            client: Client::new(),
            base_url: "https://graph.facebook.com/v18.0".to_string(),
        }
    }

    pub async fn set_access_token(&self, token: String) {
        let mut guard = self.access_token.write().await;
        *guard = Some(token);
    }

    pub async fn set_ig_user_id(&self, user_id: String) {
        let mut guard = self.ig_user_id.write().await;
        *guard = Some(user_id);
    }

    pub async fn is_authenticated(&self) -> bool {
        let guard = self.access_token.read().await;
        guard.is_some() && guard.as_ref().unwrap().len() > 10
    }

    pub async fn get_business_account(&self) -> Result<InstagramBusinessAccount> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let url = format!(
            "{}/me/accounts?fields=id,name,followers_count,media_count,profile_picture_url,instagram_business_account{{id,name,followers_count,follows_count,media_count,profile_picture_url}}&access_token={}",
            self.base_url, token
        );

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct AccountsResponse {
            data: Vec<FacebookPageAccount>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct FacebookPageAccount {
            pub id: String,
            pub name: String,
            pub followers_count: u64,
            pub media_count: u64,
            pub profile_picture_url: String,
            pub instagram_business_account: Option<InstagramAccountId>,
        }

        let result: AccountsResponse = response.json().await?;

        if let Some(page) = result.data.into_iter().next() {
            if let Some(ig_account) = page.instagram_business_account {
                Ok(InstagramBusinessAccount {
                    id: ig_account.id.clone(),
                    instagram_business_account: ig_account,
                    name: page.name,
                    followers_count: page.followers_count,
                    follows_count: 0,
                    media_count: page.media_count,
                    profile_picture_url: page.profile_picture_url,
                })
            } else {
                anyhow::bail!("No Instagram business account linked")
            }
        } else {
            anyhow::bail!("No Facebook page found")
        }
    }

    pub async fn create_image_post(&self, image_url: &str, caption: &str) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let ig_user_id = self.ig_user_id.read().await;
        let ig_user_id = ig_user_id.as_ref().context("No Instagram user ID")?;

        let params = [
            ("image_url", image_url),
            ("caption", caption),
            ("access_token", token),
        ];

        let url = format!("{}/{}/media", self.base_url, ig_user_id);

        let response = self.client.post(&url).form(&params).send().await?;

        #[derive(Deserialize)]
        struct MediaResponse {
            id: String,
            #[serde(rename = "status_code")]
            status_code: String,
        }

        let result: MediaResponse = response.json().await?;

        if result.status_code == "ACTIVE" {
            Ok(result.id)
        } else {
            anyhow::bail!("Failed to create media: {}", result.status_code)
        }
    }

    pub async fn create_video_post(
        &self,
        video_url: &str,
        caption: &str,
        cover_url: &str,
    ) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let ig_user_id = self.ig_user_id.read().await;
        let ig_user_id = ig_user_id.as_ref().context("No Instagram user ID")?;

        let params = [
            ("video_url", video_url),
            ("caption", caption),
            ("cover_url", cover_url),
            ("access_token", token),
        ];

        let url = format!("{}/{}/media", self.base_url, ig_user_id);

        let response = self.client.post(&url).form(&params).send().await?;

        #[derive(Deserialize)]
        struct MediaResponse {
            id: String,
        }

        let result: MediaResponse = response.json().await?;
        Ok(result.id)
    }

    pub async fn create_carousel_post(
        &self,
        children_ids: Vec<String>,
        caption: &str,
    ) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let ig_user_id = self.ig_user_id.read().await;
        let ig_user_id = ig_user_id.as_ref().context("No Instagram user ID")?;

        let children_str = children_ids.join(",");

        let params = [
            ("caption", caption),
            ("children_ids", &children_str),
            ("access_token", token),
        ];

        let url = format!("{}/{}/media", self.base_url, ig_user_id);

        let response = self.client.post(&url).form(&params).send().await?;

        #[derive(Deserialize)]
        struct MediaResponse {
            id: String,
        }

        let result: MediaResponse = response.json().await?;
        Ok(result.id)
    }

    pub async fn publish_media(&self, creation_id: &str) -> Result<InstagramMediaResponse> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let ig_user_id = self.ig_user_id.read().await;
        let ig_user_id = ig_user_id.as_ref().context("No Instagram user ID")?;

        let params = [("creation_id", creation_id), ("access_token", token)];

        let url = format!("{}/{}/media_publish", self.base_url, ig_user_id);

        let response = self.client.post(&url).form(&params).send().await?;

        #[derive(Deserialize)]
        struct PublishResponse {
            id: String,
        }

        let result: PublishResponse = response.json().await?;
        let id = result.id.clone();

        Ok(InstagramMediaResponse {
            id: id.clone(),
            caption: String::new(),
            permalink: format!("https://www.instagram.com/p/{}", id),
        })
    }

    pub async fn get_media(&self, limit: u32) -> Result<Vec<InstagramMedia>> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let ig_user_id = self.ig_user_id.read().await;
        let ig_user_id = ig_user_id.as_ref().context("No Instagram user ID")?;

        let url = format!(
            "{}/{}/media?fields=id,caption,media_type,media_url,permalink,timestamp,username,children{{id,media_type,media_url}}&limit={}&access_token={}",
            self.base_url, ig_user_id, limit, token
        );

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct MediaResponse {
            data: Vec<InstagramMedia>,
        }

        let result: MediaResponse = response.json().await?;
        Ok(result.data)
    }

    pub async fn get_media_insights(&self, media_id: &str) -> Result<InstagramMediaInsights> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let url = format!(
            "{}/{}/insights?metric=reach,impressions,likes,comments,shares,saves&access_token={}",
            self.base_url, media_id, token
        );

        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct InsightsResponse {
            data: Vec<InsightMetric>,
        }

        #[derive(Deserialize)]
        struct InsightMetric {
            pub name: String,
            pub values: Vec<InsightValue>,
        }

        #[derive(Deserialize)]
        struct InsightValue {
            pub value: f64,
        }

        let result: InsightsResponse = response.json().await?;

        let mut insights = InstagramMediaInsights::default();

        for metric in result.data {
            let value = metric.values.first().map(|v| v.value).unwrap_or(0.0);
            match metric.name.as_str() {
                "reach" => insights.reach = value as u64,
                "impressions" => insights.impressions = value as u64,
                "likes" => insights.likes = value as u64,
                "comments" => insights.comments = value as u64,
                "shares" => insights.shares = value as u64,
                "saves" => insights.saves = value as u64,
                _ => {}
            }
        }

        Ok(insights)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InstagramMediaInsights {
    pub reach: u64,
    pub impressions: u64,
    pub likes: u64,
    pub comments: u64,
    pub shares: u64,
    pub saves: u64,
    pub engagement: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_instagram_client_authentication() {
        let client = InstagramClient::new(
            "test_token_12345678901234567890".to_string(),
            Some("ig_user_123".to_string()),
        );
        assert!(client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_instagram_client_not_authenticated() {
        let client = InstagramClient::new("short".to_string(), None);
        assert!(!client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_set_credentials() {
        let client = InstagramClient::new("initial_token".to_string(), Some("user_1".to_string()));

        client.set_access_token("new_token".to_string()).await;
        client.set_ig_user_id("user_2".to_string()).await;

        assert!(client.is_authenticated().await);

        let ig_user_id = client.ig_user_id.read().await;
        assert_eq!(*ig_user_id, Some("user_2".to_string()));
    }
}
