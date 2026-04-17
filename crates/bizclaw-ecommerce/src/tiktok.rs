//! TikTok Shop API Integration
//!
//! Official TikTok Shop Partner API integration.
//! Requires TikTok Shop Partner account and API credentials.
//!
//! ## Setup Instructions
//! 1. Register as TikTok Shop Partner: https://partner.tiktok.com/
//! 2. Create application and get App ID + App Secret
//! 3. Implement OAuth 2.0 flow for access token
//! 4. Use sandbox mode for testing
//!
//! ## API Documentation
//! https://partner.tiktokshop.com/document/guagram/api/introduction

use anyhow::Context;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::types::{
    DailySales, InventoryItem, Order, OrderItem, OrderStatus, Product, ProductStatus, SalesReport,
    TopProduct,
};

use super::{EcommercePlatform, TiktokConfig};

#[derive(Debug)]
pub struct TiktokShop {
    config: TiktokConfig,
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
struct TiktokAuthRequest {
    app_id: String,
    app_secret: String,
    grant_type: String,
    auth_code: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TiktokAuthResponse {
    data: TiktokAuthData,
    message: String,
    code: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct TiktokAuthData {
    access_token: String,
    refresh_token: String,
    expire_in: i64,
    refresh_expire_in: i64,
    open_id: String,
}

#[derive(Debug, Serialize)]
struct TiktokApiRequest {
    access_token: String,
    app_id: String,
    shop_id: String,
}

#[derive(Debug, Deserialize)]
struct TiktokApiResponse<T> {
    data: T,
    message: String,
    code: i32,
}

#[derive(Debug, Deserialize)]
struct TiktokOrderList {
    orders: Vec<TiktokOrder>,
    more: bool,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TiktokOrder {
    order_id: String,
    status: String,
    recipient: Option<TiktokRecipient>,
    total_amount: f64,
    shipping_fee: f64,
    discount: f64,
    items: Vec<TiktokOrderItem>,
    create_time: i64,
    update_time: i64,
    remark: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TiktokRecipient {
    full_name: String,
    phone_number: Option<String>,
    address: Option<TiktokAddress>,
}

#[derive(Debug, Deserialize)]
struct TiktokAddress {
    full_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TiktokOrderItem {
    item_id: String,
    item_name: String,
    sku_id: Option<String>,
    sku_name: Option<String>,
    quantity: i32,
    unit_price: f64,
    discount: f64,
    item_total: f64,
    image: Option<TiktokImage>,
}

#[derive(Debug, Clone, Deserialize)]
struct TiktokImage {
    url: String,
}

#[derive(Debug, Deserialize)]
struct TiktokProductList {
    products: Vec<TiktokProduct>,
    more: bool,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TiktokProduct {
    product_id: String,
    product_name: String,
    description: Option<String>,
    category: Option<TiktokCategory>,
    sku_list: Vec<TiktokSku>,
    status: String,
    rating: Option<f32>,
    sales: Option<i32>,
    create_time: i64,
    update_time: i64,
}

#[derive(Debug, Deserialize)]
struct TiktokCategory {
    category_id: String,
    category_name: String,
}

#[derive(Debug, Deserialize)]
struct TiktokSku {
    sku_id: String,
    price: f64,
    original_price: Option<f64>,
    stock_info: Option<TiktokStock>,
    image: Option<TiktokImage>,
}

#[derive(Debug, Deserialize)]
struct TiktokStock {
    #[serde(rename = "total")]
    total: i32,
}

impl TiktokShop {
    pub fn new(config: TiktokConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://open.tiktokapis.com/v2".to_string(),
        }
    }

    pub fn with_sandbox(config: TiktokConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://open-sandbox.tiktokapis.com/v2".to_string(),
        }
    }

    fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
    }

    fn map_status(tiktok_status: &str) -> OrderStatus {
        match tiktok_status.to_lowercase().as_str() {
            "100" | "pending" => OrderStatus::Pending,
            "111" | "confirmed" => OrderStatus::Confirmed,
            "112" | "processing" => OrderStatus::Processing,
            "200" | "shipped" => OrderStatus::Shipped,
            "300" | "delivered" | "completed" => OrderStatus::Delivered,
            "400" | "cancelled" => OrderStatus::Cancelled,
            "500" | "returned" => OrderStatus::Returned,
            "600" | "refunded" => OrderStatus::Refunded,
            _ => OrderStatus::Unknown,
        }
    }

    fn map_product_status(status: &str) -> ProductStatus {
        match status.to_lowercase().as_str() {
            "0" | "active" => ProductStatus::Active,
            "1" | "inactive" => ProductStatus::Inactive,
            "2" | "deleted" => ProductStatus::Deleted,
            _ => ProductStatus::Unknown,
        }
    }

    fn convert_order(&self, tiktok_order: TiktokOrder) -> Order {
        Order {
            id: tiktok_order.order_id,
            platform: "tiktok_shop".to_string(),
            status: Self::map_status(&tiktok_order.status),
            customer_name: tiktok_order
                .recipient
                .as_ref()
                .map(|r| r.full_name.clone())
                .unwrap_or_default(),
            customer_phone: tiktok_order
                .recipient
                .as_ref()
                .and_then(|r| r.phone_number.clone()),
            customer_address: tiktok_order
                .recipient
                .as_ref()
                .and_then(|r| r.address.as_ref())
                .and_then(|a| a.full_address.clone()),
            total_amount: tiktok_order.total_amount,
            shipping_fee: tiktok_order.shipping_fee,
            discount: tiktok_order.discount,
            items: tiktok_order
                .items
                .into_iter()
                .map(|item| OrderItem {
                    id: format!(
                        "{}_{}",
                        item.item_id,
                        item.sku_id.clone().unwrap_or_default()
                    ),
                    product_id: item.item_id,
                    product_name: item.item_name,
                    sku: item.sku_id,
                    quantity: item.quantity,
                    unit_price: item.unit_price,
                    discount: item.discount,
                    total_price: item.item_total,
                    image_url: item.image.map(|i| i.url),
                })
                .collect(),
            created_at: Self::timestamp_to_datetime(tiktok_order.create_time),
            updated_at: Self::timestamp_to_datetime(tiktok_order.update_time),
            notes: tiktok_order.remark,
        }
    }

    fn convert_product(&self, tiktok_product: TiktokProduct) -> Product {
        let main_sku = tiktok_product.sku_list.first();
        let total_stock: i32 = tiktok_product
            .sku_list
            .iter()
            .filter_map(|s| s.stock_info.as_ref().map(|st| st.total))
            .sum();

        Product {
            id: tiktok_product.product_id,
            platform: "tiktok_shop".to_string(),
            name: tiktok_product.product_name,
            description: tiktok_product.description,
            category: tiktok_product.category.map(|c| c.category_name),
            sku: main_sku.map(|s| s.sku_id.clone()),
            price: main_sku.map(|s| s.price).unwrap_or(0.0),
            original_price: main_sku.and_then(|s| s.original_price),
            stock: total_stock,
            images: main_sku
                .and_then(|s| s.image.as_ref())
                .map(|i| vec![i.url.clone()])
                .unwrap_or_default(),
            status: Self::map_product_status(&tiktok_product.status),
            rating: tiktok_product.rating,
            sold_count: tiktok_product.sales.unwrap_or(0),
            created_at: Self::timestamp_to_datetime(tiktok_product.create_time),
            updated_at: Self::timestamp_to_datetime(tiktok_product.update_time),
        }
    }
}

#[async_trait::async_trait]
impl EcommercePlatform for TiktokShop {
    fn name(&self) -> &str {
        "tiktok_shop"
    }

    fn is_authenticated(&self) -> bool {
        self.config.access_token.is_some()
    }

    async fn authenticate(&mut self) -> anyhow::Result<()> {
        let auth_url = format!("{}/oauth/token/", self.base_url);

        let request = if let Some(refresh_token) = &self.config.access_token {
            serde_json::to_string(&serde_json::json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret,
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
            }))
        } else {
            anyhow::bail!("No refresh token available. Need initial OAuth flow.");
        };

        let response = self
            .client
            .post(&auth_url)
            .header("Content-Type", "application/json")
            .body(request?)
            .send()
            .await
            .context("Failed to authenticate with TikTok")?;

        let auth_response: TiktokAuthResponse = response
            .json()
            .await
            .context("Failed to parse TikTok auth response")?;

        if auth_response.code.unwrap_or(0) != 0 {
            anyhow::bail!("TikTok auth failed: {}", auth_response.message);
        }

        self.config.access_token = Some(auth_response.data.access_token);
        tracing::info!("TikTok Shop authentication successful");
        Ok(())
    }

    async fn get_orders(&self, status: Option<&str>) -> anyhow::Result<Vec<Order>> {
        let access_token = self
            .config
            .access_token
            .as_ref()
            .context("Not authenticated")?;

        let shop_id = self
            .config
            .shop_id
            .as_ref()
            .context("No shop_id configured")?;

        let mut url = format!(
            "{}/order/list/get/?access_token={}",
            self.base_url, access_token
        );

        let mut body = serde_json::json!({
            "shop_id": shop_id,
            "page_size": 100,
        });

        if let Some(s) = status {
            body["order_status"] = serde_json::json!(s);
        }

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to fetch TikTok orders")?;

        let api_response: TiktokApiResponse<TiktokOrderList> = response
            .json()
            .await
            .context("Failed to parse TikTok order response")?;

        if api_response.code != 0 {
            anyhow::bail!("TikTok API error: {}", api_response.message);
        }

        let orders: Vec<Order> = api_response
            .data
            .orders
            .into_iter()
            .map(|o| self.convert_order(o))
            .collect();

        Ok(orders)
    }

    async fn get_products(&self) -> anyhow::Result<Vec<Product>> {
        let access_token = self
            .config
            .access_token
            .as_ref()
            .context("Not authenticated")?;

        let shop_id = self
            .config
            .shop_id
            .as_ref()
            .context("No shop_id configured")?;

        let url = format!(
            "{}/product/list/get/?access_token={}",
            self.base_url, access_token
        );

        let body = serde_json::json!({
            "shop_id": shop_id,
            "page_size": 100,
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to fetch TikTok products")?;

        let api_response: TiktokApiResponse<TiktokProductList> = response
            .json()
            .await
            .context("Failed to parse TikTok product response")?;

        if api_response.code != 0 {
            anyhow::bail!("TikTok API error: {}", api_response.message);
        }

        let products: Vec<Product> = api_response
            .data
            .products
            .into_iter()
            .map(|p| self.convert_product(p))
            .collect();

        Ok(products)
    }

    async fn get_inventory(
        &self,
        product_ids: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<InventoryItem>> {
        let access_token = self
            .config
            .access_token
            .as_ref()
            .context("Not authenticated")?;

        let shop_id = self
            .config
            .shop_id
            .as_ref()
            .context("No shop_id configured")?;

        let url = format!(
            "{}/inventory/list/?access_token={}",
            self.base_url, access_token
        );

        let body = if let Some(ids) = product_ids {
            serde_json::json!({
                "shop_id": shop_id,
                "product_ids": ids,
            })
        } else {
            serde_json::json!({
                "shop_id": shop_id,
            })
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to fetch TikTok inventory")?;

        #[derive(Deserialize)]
        struct InventoryResponse {
            data: Vec<InventoryItem>,
        }

        let api_response: TiktokApiResponse<InventoryResponse> = response
            .json()
            .await
            .context("Failed to parse TikTok inventory response")?;

        if api_response.code != 0 {
            anyhow::bail!("TikTok API error: {}", api_response.message);
        }

        Ok(api_response.data.data)
    }

    async fn update_inventory(&self, product_id: &str, quantity: i32) -> anyhow::Result<()> {
        let access_token = self
            .config
            .access_token
            .as_ref()
            .context("Not authenticated")?;

        let shop_id = self
            .config
            .shop_id
            .as_ref()
            .context("No shop_id configured")?;

        let url = format!(
            "{}/inventory/update/?access_token={}",
            self.base_url, access_token
        );

        let body = serde_json::json!({
            "shop_id": shop_id,
            "product_id": product_id,
            "quantity": quantity,
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to update TikTok inventory")?;

        let api_response: TiktokApiResponse<()> = response
            .json()
            .await
            .context("Failed to parse TikTok inventory update response")?;

        if api_response.code != 0 {
            anyhow::bail!("TikTok API error: {}", api_response.message);
        }

        Ok(())
    }
}

impl TiktokShop {
    pub async fn get_sales_report(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<SalesReport> {
        let orders = self.get_orders(None).await?;

        let filtered_orders: Vec<&Order> = orders
            .iter()
            .filter(|o| {
                let date = o.created_at.format("%Y-%m-%d").to_string();
                date >= start_date.to_string() && date <= end_date.to_string()
            })
            .collect();

        let total_orders = filtered_orders.len() as i32;
        let total_revenue: f64 = filtered_orders.iter().map(|o| o.total_amount).sum();
        let total_shipping: f64 = filtered_orders.iter().map(|o| o.shipping_fee).sum();
        let total_discount: f64 = filtered_orders.iter().map(|o| o.discount).sum();

        let mut product_sales: std::collections::HashMap<String, (String, i32, f64)> =
            std::collections::HashMap::new();

        for order in &filtered_orders {
            for item in &order.items {
                let entry = product_sales
                    .entry(item.product_id.clone())
                    .or_insert_with(|| (item.product_name.clone(), 0, 0.0));
                entry.1 += item.quantity;
                entry.2 += item.total_price;
            }
        }

        let mut top_products: Vec<TopProduct> = product_sales
            .into_iter()
            .map(|(id, (name, qty, rev))| TopProduct {
                product_id: id,
                product_name: name,
                quantity_sold: qty,
                revenue: rev,
            })
            .collect();

        top_products.sort_by(|a, b| b.revenue.partial_cmp(&a.revenue).unwrap());
        top_products.truncate(10);

        let mut daily_sales: std::collections::HashMap<String, (i32, f64)> =
            std::collections::HashMap::new();

        for order in &filtered_orders {
            let date = order.created_at.format("%Y-%m-%d").to_string();
            let entry = daily_sales.entry(date).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += order.total_amount;
        }

        let daily_breakdown: Vec<DailySales> = daily_sales
            .into_iter()
            .map(|(date, (orders, revenue))| DailySales {
                date,
                orders,
                revenue,
            })
            .collect();

        Ok(SalesReport {
            platform: "tiktok_shop".to_string(),
            period_start: chrono::DateTime::parse_from_rfc3339(&format!(
                "{}T00:00:00Z",
                start_date
            ))
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
            period_end: chrono::DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", end_date))
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            total_orders,
            total_revenue,
            total_shipping_fee: total_shipping,
            total_discount,
            total_refunds: 0.0,
            net_revenue: total_revenue - total_shipping - total_discount,
            top_products,
            daily_breakdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiktok_status_mapping() {
        assert_eq!(TiktokShop::map_status("100"), OrderStatus::Pending);
        assert_eq!(TiktokShop::map_status("200"), OrderStatus::Shipped);
        assert_eq!(TiktokShop::map_status("300"), OrderStatus::Delivered);
    }

    #[test]
    fn test_product_status_mapping() {
        assert_eq!(TiktokShop::map_product_status("0"), ProductStatus::Active);
        assert_eq!(TiktokShop::map_product_status("1"), ProductStatus::Inactive);
    }

    #[tokio::test]
    async fn test_tiktok_shop_creation() {
        let config = TiktokConfig {
            app_id: "test_app_id".to_string(),
            app_secret: "test_secret".to_string(),
            access_token: None,
            shop_id: None,
        };

        let shop = TiktokShop::new(config);
        assert_eq!(shop.name(), "tiktok_shop");
        assert!(!shop.is_authenticated());
    }
}
