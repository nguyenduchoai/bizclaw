use crate::types::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct TikTokClient {
    client_key: String,
    client_secret: String,
    access_token: Arc<RwLock<Option<String>>>,
    open_id: Arc<RwLock<Option<String>>>,
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokAuthResponse {
    pub data: TikTokAuthData,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokAuthData {
    pub access_token: String,
    pub refresh_token: String,
    pub open_id: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokVideo {
    pub video_id: String,
    pub title: String,
    pub description: String,
    pub cover_image_url: String,
    pub share_url: String,
    pub create_time: u64,
    pub status: TikTokVideoStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TikTokVideoStatus {
    Published,
    Processing,
    Private,
    Removed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokUser {
    pub open_id: String,
    pub union_id: Option<String>,
    pub nickname: String,
    pub avatar_url: String,
    pub is_verified: bool,
    pub follower_count: u64,
    pub following_count: u64,
    pub likes_count: u64,
    pub video_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokPostRequest {
    pub video_description: String,
    pub post_mode: String,
    pub auto_add_music: bool,
    pub privacy_level: String,
    pub disable_comment: bool,
    pub disable_share: bool,
    pub video_category: Option<u32>,
    pub game_tag: Option<String>,
    pub mention_user_ids: Option<Vec<String>>,
    pub hashtag_ids: Option<Vec<u64>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokPostResponse {
    pub video_id: String,
    pub upload_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokUploadInitResponse {
    pub video_id: String,
    pub upload_url: String,
    pub expire_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokVideoQuery {
    pub video_ids: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokVideoListResponse {
    pub video_list: Vec<TikTokVideo>,
}

impl TikTokClient {
    pub fn new(client_key: String, client_secret: String) -> Self {
        Self {
            client_key,
            client_secret,
            access_token: Arc::new(RwLock::new(None)),
            open_id: Arc::new(RwLock::new(None)),
            client: Client::new(),
            base_url: "https://open.tiktokapis.com".to_string(),
        }
    }

    pub fn new_with_url(base_url: &str) -> Self {
        Self {
            client_key: String::new(),
            client_secret: String::new(),
            access_token: Arc::new(RwLock::new(None)),
            open_id: Arc::new(RwLock::new(None)),
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn set_credentials(&self, access_token: String, open_id: String) {
        let mut token_guard = self.access_token.write().await;
        *token_guard = Some(access_token);

        let mut openid_guard = self.open_id.write().await;
        *openid_guard = Some(open_id);
    }

    pub async fn is_authenticated(&self) -> bool {
        let guard = self.access_token.read().await;
        guard.is_some() && guard.as_ref().unwrap().len() > 10
    }

    pub fn generate_auth_url(&self, redirect_uri: &str, state: &str) -> String {
        format!(
            "https://www.tiktok.com/v2/auth/authorize/?client_key={}&scope=user.info.basic,video.upload,video.publish&response_type=code&redirect_uri={}&state={}",
            self.client_key, redirect_uri, state
        )
    }

    pub async fn get_access_token(&self, code: &str, redirect_uri: &str) -> Result<TikTokAuthData> {
        let params = [
            ("client_key", self.client_key.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ];

        let response = self
            .client
            .post("https://open.tiktokapis.com/v2/oauth/token/")
            .form(&params)
            .send()
            .await?;

        let data: TikTokAuthResponse = response.json().await?;

        if data.message == "success" {
            self.set_credentials(data.data.access_token.clone(), data.data.open_id.clone())
                .await;
            Ok(data.data)
        } else {
            anyhow::bail!("TikTok auth failed: {}", data.message)
        }
    }

    pub async fn get_user_info(&self) -> Result<TikTokUser> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let query = r#"{"fields": ["open_id", "union_id", "avatar_url", "display_name", "is_verified", "follower_count", "following_count", "likes_count", "video_count"]}"#;

        let response = self
            .client
            .post(format!("{}/v2/user/info/", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(query)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct UserResponse {
            data: TikTokUser,
        }

        let result: UserResponse = response.json().await?;
        Ok(result.data)
    }

    pub async fn upload_video_init(
        &self,
        file_size: u64,
        file_name: &str,
    ) -> Result<TikTokUploadInitResponse> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let query = serde_json::json!({
            "upload_type": "video",
            "video_info": {
                "file_size": file_size,
                "file_name": file_name,
            }
        });

        let response = self
            .client
            .post(format!("{}/v2/video/init/", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&query)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct InitResponse {
            data: TikTokUploadInitResponse,
        }

        let result: InitResponse = response.json().await?;
        Ok(result.data)
    }

    pub async fn upload_video_chunk(
        &self,
        upload_url: &str,
        video_data: &[u8],
        part_number: u32,
    ) -> Result<()> {
        let response = self
            .client
            .put(upload_url)
            .header("Content-Type", "video/mp4")
            .header("Content-Length", video_data.len().to_string())
            .header("Content-MD5", format!("{:x}", Sha256::digest(video_data)))
            .body(video_data.to_vec())
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            anyhow::bail!("Upload failed with status: {}", status)
        }
    }

    pub async fn publish_video(
        &self,
        video_id: &str,
        title: &str,
        description: &str,
    ) -> Result<String> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let post_request = TikTokPostRequest {
            video_description: format!("{}\n\n{}", title, description),
            post_mode: "self".to_string(),
            auto_add_music: false,
            privacy_level: "public".to_string(),
            disable_comment: false,
            disable_share: false,
            video_category: None,
            game_tag: None,
            mention_user_ids: None,
            hashtag_ids: None,
        };

        let response = self
            .client
            .post(format!("{}/v2/video/upload/search/", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&post_request)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct PublishResponse {
            data: PublishData,
        }

        #[derive(Deserialize)]
        struct PublishData {
            video_id: String,
        }

        let result: PublishResponse = response.json().await?;
        Ok(result.data.video_id)
    }

    pub async fn get_videos(&self, max_count: u32) -> Result<Vec<TikTokVideo>> {
        let token = self.access_token.read().await;
        let token = token.as_ref().context("No access token")?;

        let query = serde_json::json!({
            "max_count": max_count,
            "fields": ["id", "title", "description", "cover_image_url", "share_url", "create_time", "video_status"]
        });

        let response = self
            .client
            .post(format!("{}/v2/video/list/", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&query)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct ListResponse {
            data: TikTokVideoListResponse,
        }

        let result: ListResponse = response.json().await?;
        Ok(result.data.video_list)
    }
}

pub struct TikTokShopClient {
    client_key: String,
    client_secret: String,
    access_token: Option<String>,
    app_id: String,
    client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokShopProduct {
    pub product_id: String,
    pub product_name: String,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub stock: u64,
    pub images: Vec<String>,
    pub status: ProductStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductStatus {
    Active,
    Inactive,
    Deleted,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TikTokShopOrder {
    pub order_id: String,
    pub create_time: u64,
    pub total_amount: f64,
    pub currency: String,
    pub status: OrderStatus,
    pub buyer_info: BuyerInfo,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuyerInfo {
    pub buyer_id: String,
    pub name: String,
    pub phone: Option<String>,
    pub address: Address,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Address {
    pub full_address: String,
    pub district: String,
    pub city: String,
    pub country: String,
}

impl TikTokShopClient {
    pub fn new(client_key: String, client_secret: String, app_id: String) -> Self {
        Self {
            client_key,
            client_secret,
            access_token: None,
            app_id,
            client: Client::new(),
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.access_token = Some(token);
        self
    }

    pub async fn get_products(&self) -> Result<Vec<TikTokShopProduct>> {
        let token = self.access_token.as_ref().context("No access token")?;

        let response = self
            .client
            .get("https://open-api.tiktokglobalshop.com/202309/products")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        #[derive(Deserialize)]
        struct ProductsResponse {
            data: ProductsData,
        }

        #[derive(Deserialize)]
        struct ProductsData {
            products: Vec<TikTokShopProduct>,
        }

        let result: ProductsResponse = response.json().await?;
        Ok(result.data.products)
    }

    pub async fn create_product(&self, product: &TikTokShopProduct) -> Result<String> {
        let token = self.access_token.as_ref().context("No access token")?;

        let response = self
            .client
            .post("https://open-api.tiktokglobalshop.com/202309/products")
            .header("Authorization", format!("Bearer {}", token))
            .json(product)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct CreateResponse {
            data: CreateData,
        }

        #[derive(Deserialize)]
        struct CreateData {
            product_id: String,
        }

        let result: CreateResponse = response.json().await?;
        Ok(result.data.product_id)
    }

    pub async fn get_orders(&self, page_size: u32) -> Result<Vec<TikTokShopOrder>> {
        let token = self.access_token.as_ref().context("No access token")?;

        let response = self
            .client
            .get(&format!(
                "https://open-api.tiktokglobalshop.com/202309/orders?page_size={}",
                page_size
            ))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        #[derive(Deserialize)]
        struct OrdersResponse {
            data: OrdersData,
        }

        #[derive(Deserialize)]
        struct OrdersData {
            orders: Vec<TikTokShopOrder>,
        }

        let result: OrdersResponse = response.json().await?;
        Ok(result.data.orders)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tiktok_client_authentication() {
        let client = TikTokClient::new(
            "test_client_key".to_string(),
            "test_client_secret".to_string(),
        );
        assert!(!client.is_authenticated().await);
    }

    #[test]
    fn test_generate_auth_url() {
        let client = TikTokClient::new("my_client_key".to_string(), "my_secret".to_string());

        let url = client.generate_auth_url("https://myapp.com/callback", "random_state_string");

        assert!(url.contains("client_key=my_client_key"));
        assert!(url.contains("redirect_uri=https://myapp.com/callback"));
    }

    #[test]
    fn test_tiktok_shop_client() {
        let client = TikTokShopClient::new(
            "shop_client_key".to_string(),
            "shop_secret".to_string(),
            "app_12345".to_string(),
        );

        assert!(client.access_token.is_none());

        let client_with_token = client.with_token("my_access_token".to_string());
        assert!(client_with_token.access_token.is_some());
    }
}
