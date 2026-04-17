//! # BizClaw E-commerce API Integrations
//!
//! Official API integrations for Vietnamese e-commerce platforms:
//! - TikTok Shop API
//! - Shopee API
//!
//! ## Compliance Notes
//! - All data collection uses official APIs only
//! - Respects platform rate limits
//! - Implements proper authentication (OAuth/API keys)
//! - No scraping or unauthorized data collection

pub mod shopee;
pub mod tiktok;
pub mod types;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcommerceConfig {
    pub tiktok: Option<TiktokConfig>,
    pub shopee: Option<ShopeeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiktokConfig {
    pub app_id: String,
    pub app_secret: String,
    pub access_token: Option<String>,
    pub shop_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopeeConfig {
    pub partner_id: i64,
    pub shop_id: i64,
    pub api_key: String,
    pub secret_key: String,
}

#[async_trait::async_trait]
pub trait EcommercePlatform: Send + Sync {
    fn name(&self) -> &str;
    fn is_authenticated(&self) -> bool;
    async fn authenticate(&mut self) -> anyhow::Result<()>;
    async fn get_orders(&self, status: Option<&str>) -> anyhow::Result<Vec<types::Order>>;
    async fn get_products(&self) -> anyhow::Result<Vec<types::Product>>;
    async fn get_inventory(
        &self,
        product_ids: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<types::InventoryItem>>;
    async fn update_inventory(&self, product_id: &str, quantity: i32) -> anyhow::Result<()>;
}
