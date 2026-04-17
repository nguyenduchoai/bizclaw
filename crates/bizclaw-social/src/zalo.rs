use crate::types::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct ZaloClient {
    access_token: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
struct ZaloApiRequest {
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ZaloApiResponse<T> {
    pub error: Option<i32>,
    pub message: Option<String>,
    #[serde(flatten)]
    pub data: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZaloSendMessageRequest {
    pub recipient: ZaloRecipient,
    pub message: ZaloMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZaloRecipient {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ZaloMessage {
    Text {
        text: String,
    },
    Image {
        attachment_id: String,
    },
    Video {
        attachment_id: String,
    },
    Link {
        caption: String,
        description: String,
        link: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZaloFollower {
    pub user_id: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub is_follower: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZaloPageInfo {
    pub page_id: String,
    pub name: String,
    pub followers_count: u64,
    pub following_count: u64,
}

impl ZaloClient {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token: Arc::new(RwLock::new(Some(access_token))),
            client: Client::new(),
            base_url: "https://openapi.zalo.me".to_string(),
        }
    }

    pub fn new_with_url(base_url: &str) -> Self {
        Self {
            access_token: Arc::new(RwLock::new(None)),
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn set_access_token(&self, token: String) {
        let mut guard = self.access_token.write().await;
        *guard = Some(token);
    }

    pub async fn is_authenticated(&self) -> bool {
        let guard = self.access_token.read().await;
        guard.is_some() && guard.as_ref().unwrap().len() > 10
    }

    pub async fn get_page_info(&self) -> Result<ZaloPageInfo> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let url = format!("{}/v3.0/page/getprofile", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("access_token", token)
            .send()
            .await?;

        let data: ZaloApiResponse<ZaloPageInfo> = response.json().await?;

        data.data.context("Failed to get page info")
    }

    pub async fn send_text_message(
        &self,
        user_id: &str,
        text: &str,
    ) -> Result<ZaloSendMessageResponse> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let payload = ZaloSendMessageRequest {
            recipient: ZaloRecipient {
                user_id: user_id.to_string(),
            },
            message: ZaloMessage::Text {
                text: text.to_string(),
            },
        };

        let url = format!("{}/v3.0/oa/message/text", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("access_token", token)
            .json(&payload)
            .send()
            .await?;

        let data: ZaloApiResponse<ZaloSendMessageResponse> = response.json().await?;

        match data.error {
            Some(0) | None => Ok(data.data.unwrap_or_default()),
            Some(e) => anyhow::bail!("Zalo API error: {} - {:?}", e, data.message),
        }
    }

    pub async fn send_image_message(
        &self,
        user_id: &str,
        attachment_id: &str,
    ) -> Result<ZaloSendMessageResponse> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let payload = ZaloSendMessageRequest {
            recipient: ZaloRecipient {
                user_id: user_id.to_string(),
            },
            message: ZaloMessage::Image {
                attachment_id: attachment_id.to_string(),
            },
        };

        let url = format!("{}/v3.0/oa/message/image", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("access_token", token)
            .json(&payload)
            .send()
            .await?;

        let data: ZaloApiResponse<ZaloSendMessageResponse> = response.json().await?;

        match data.error {
            Some(0) | None => Ok(data.data.unwrap_or_default()),
            Some(e) => anyhow::bail!("Zalo API error: {} - {:?}", e, data.message),
        }
    }

    pub async fn upload_image(&self, image_data: &[u8], filename: &str) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let url = format!("{}/v3.0/oa/upload/image", self.base_url);

        let part =
            reqwest::multipart::Part::bytes(image_data.to_vec()).file_name(filename.to_string());

        let form = reqwest::multipart::Form::new().part("file", part);

        let response = self
            .client
            .post(&url)
            .header("access_token", token)
            .multipart(form)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct UploadResponse {
            data: UploadedImage,
        }

        #[derive(Deserialize)]
        struct UploadedImage {
            attachment_id: String,
        }

        let data: ZaloApiResponse<UploadedImage> = response.json().await?;

        data.data
            .map(|d| d.attachment_id)
            .context("Failed to upload image")
    }

    pub async fn get_followers(&self, offset: u32, limit: u32) -> Result<Vec<ZaloFollower>> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let url = format!(
            "{}/v3.0/oa/contacts/followers?offset={}&count={}",
            self.base_url, offset, limit
        );

        let response = self
            .client
            .get(&url)
            .header("access_token", token)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct FollowersResponse {
            followers: Vec<ZaloFollower>,
        }

        let data: ZaloApiResponse<FollowersResponse> = response.json().await?;

        data.data
            .map(|d| d.followers)
            .context("Failed to get followers")
    }

    pub async fn create_qr_link(&self, payload: &str) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        #[derive(Serialize)]
        struct QrRequest<'a> {
            data: &'a str,
        }

        #[derive(Deserialize)]
        struct QrResponse {
            url: String,
        }

        let url = format!("{}/v3.0/qrcode", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("access_token", token)
            .json(&QrRequest { data: payload })
            .send()
            .await?;

        let data: ZaloApiResponse<QrResponse> = response.json().await?;

        data.data.map(|d| d.url).context("Failed to create QR link")
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ZaloSendMessageResponse {
    pub success: bool,
    pub msg_id: Option<String>,
}

pub struct ZaloOABuilder {
    client: ZaloClient,
}

impl ZaloOABuilder {
    pub fn new() -> Self {
        Self {
            client: ZaloClient::new_with_url("https://openapi.zalo.me"),
        }
    }

    pub fn access_token(mut self, token: &str) -> Self {
        self
    }

    pub fn base_url(mut self, url: &str) -> Self {
        self.client.base_url = url.to_string();
        self
    }

    pub fn build(self) -> ZaloClient {
        self.client
    }
}

impl Default for ZaloOABuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_zalo_client_authentication() {
        let client = ZaloClient::new("test_token_12345".to_string());
        assert!(client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_zalo_client_not_authenticated() {
        let client = ZaloClient::new_with_url("https://openapi.zalo.me");
        assert!(!client.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_set_access_token() {
        let client = ZaloClient::new_with_url("https://openapi.zalo.me");
        assert!(!client.is_authenticated().await);

        client.set_access_token("new_token_12345".to_string()).await;
        assert!(client.is_authenticated().await);
    }

    #[test]
    fn test_zalo_oa_builder() {
        let client = ZaloOABuilder::new()
            .base_url("https://custom.zalo.me")
            .build();

        assert_eq!(client.base_url, "https://custom.zalo.me");
    }
}
