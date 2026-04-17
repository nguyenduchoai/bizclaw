//! Shopee API Integration
//!
//! Official Shopee Partner API integration for Vietnam market.
//! Requires Shopee Partner account and API credentials.
//!
//! ## Setup Instructions
//! 1. Register as Shopee Partner: https://partner.shopee.com/
//! 2. Create application and get Partner ID + Partner Key
//! 3. Implement signature-based authentication
//! 4. Use sandbox mode for testing
//!
//! ## API Documentation
//! https://open.shopee.com/document/developer-tutorial/process-flow/api-authorization

use std::time::SystemTime;

use anyhow::Context;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::types::{
    DailySales, InventoryItem, Order, OrderItem, OrderStatus, Product, ProductStatus, SalesReport,
    TopProduct,
};

use super::{EcommercePlatform, ShopeeConfig};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug)]
pub struct ShopeeApi {
    config: ShopeeConfig,
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
struct ShopeeRequest<T> {
    partner_id: i64,
    shopid: i64,
    timestamp: i64,
    #[serde(flatten)]
    body: T,
}

#[derive(Debug, Deserialize)]
struct ShopeeResponse<T> {
    error: Option<String>,
    message: Option<String>,
    response: Option<T>,
    request_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct OrderListRequest {
    order_status: Option<String>,
    page_size: Option<i32>,
    cursor: Option<String>,
    create_time_from: Option<i64>,
    create_time_to: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OrderListResponse {
    order_list: Vec<ShopeeOrder>,
    next_cursor: Option<String>,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct ShopeeOrder {
    order_sn: String,
    order_status: String,
    buyer_username: Option<String>,
    recipient_address: Option<ShopeeAddress>,
    amount: f64,
    shipping_fee: f64,
    discount: f64,
    items: Option<Vec<ShopeeOrderItem>>,
    create_time: i64,
    update_time: i64,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopeeAddress {
    full_address: Option<String>,
    receiver_name: Option<String>,
    phone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShopeeOrderItem {
    itemid: i64,
    item_name: Option<String>,
    modelid: Option<i64>,
    model_name: Option<String>,
    quantity: i32,
    price: f64,
    discount_price: Option<f64>,
    item_total: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ProductListRequest {
    page_size: Option<i32>,
    page_token: Option<String>,
    update_time_from: Option<i64>,
    update_time_to: Option<i64>,
    #[serde(rename = "S")]
    status: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ProductListResponse {
    product_list: Vec<ShopeeProduct>,
    next_page_token: Option<String>,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct ShopeeProduct {
    product_id: i64,
    product_name: String,
    description: Option<String>,
    category_id: Option<i64>,
    model_list: Option<Vec<ShopeeModel>>,
    status: i32,
    rating_star: Option<f32>,
    historical_sold: Option<i32>,
    create_time: i64,
    update_time: i64,
    image: Option<ShopeeImage>,
}

#[derive(Debug, Deserialize)]
struct ShopeeModel {
    model_id: i64,
    model_name: Option<String>,
    price: f64,
    original_price: Option<f64>,
    stock_info: Option<Vec<ShopeeStock>>,
}

#[derive(Debug, Deserialize)]
struct ShopeeStock {
    #[serde(rename = "normal_stock")]
    normal_stock: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ShopeeImage {
    image_url_list: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct InventoryUpdateRequest {
    item_id: i64,
    model_id: Option<i64>,
    stock: i32,
}

impl ShopeeApi {
    pub fn new(config: ShopeeConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://partner.shopee.com".to_string(),
        }
    }

    pub fn with_staging(config: ShopeeConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://stg-open.shopee.com".to_string(),
        }
    }

    fn generate_signature(&self, path: &str, body: &str) -> String {
        let message = format!("{}{}{}", self.config.partner_id, path, body);
        let mut mac = HmacSha256::new_from_slice(self.config.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            result.into_bytes(),
        )
    }

    fn map_status(shopee_status: &str) -> OrderStatus {
        match shopee_status.to_uppercase().as_str() {
            "UNPAID" | "0" => OrderStatus::Pending,
            "PAID" | "1" => OrderStatus::Confirmed,
            "PROCESSING" | "2" => OrderStatus::Processing,
            "SHIPPED" | "3" => OrderStatus::Shipped,
            "COMPLETED" | "4" => OrderStatus::Delivered,
            "CANCELLED" | "5" => OrderStatus::Cancelled,
            "INCONSISTENT_STATUS" | "6" => OrderStatus::Unknown,
            _ => OrderStatus::Unknown,
        }
    }

    fn map_product_status(status: i32) -> ProductStatus {
        match status {
            1 => ProductStatus::Active,
            2 => ProductStatus::Inactive,
            3 | 4 => ProductStatus::Deleted,
            _ => ProductStatus::Unknown,
        }
    }

    fn timestamp_to_datetime(ts: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
    }

    fn convert_order(&self, shopee_order: ShopeeOrder) -> Order {
        let items: Vec<OrderItem> = shopee_order
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|item| OrderItem {
                id: format!("{}_{}", item.itemid, item.modelid.unwrap_or(0)),
                product_id: item.itemid.to_string(),
                product_name: item
                    .item_name
                    .unwrap_or_else(|| "Unknown Product".to_string()),
                sku: item.modelid.map(|id| id.to_string()),
                quantity: item.quantity,
                unit_price: item.price / 100000.0,
                discount: (item.discount_price.unwrap_or(0.0)) / 100000.0,
                total_price: item.item_total.unwrap_or(item.price) / 100000.0,
                image_url: None,
            })
            .collect();

        Order {
            id: shopee_order.order_sn,
            platform: "shopee".to_string(),
            status: Self::map_status(&shopee_order.order_status),
            customer_name: shopee_order
                .recipient_address
                .as_ref()
                .and_then(|a| a.receiver_name.clone())
                .unwrap_or_else(|| shopee_order.buyer_username.clone().unwrap_or_default()),
            customer_phone: shopee_order
                .recipient_address
                .as_ref()
                .and_then(|a| a.phone.clone()),
            customer_address: shopee_order.recipient_address.and_then(|a| a.full_address),
            total_amount: shopee_order.amount / 100000.0,
            shipping_fee: shopee_order.shipping_fee / 100000.0,
            discount: shopee_order.discount / 100000.0,
            items,
            created_at: Self::timestamp_to_datetime(shopee_order.create_time),
            updated_at: Self::timestamp_to_datetime(shopee_order.update_time),
            notes: shopee_order.note,
        }
    }

    fn convert_product(&self, shopee_product: ShopeeProduct) -> Product {
        let main_model = shopee_product.model_list.as_ref().and_then(|m| m.first());
        let total_stock: i32 = shopee_product
            .model_list
            .as_ref()
            .map(|models| {
                models
                    .iter()
                    .filter_map(|m| m.stock_info.as_ref())
                    .flatten()
                    .filter_map(|s| s.normal_stock)
                    .sum()
            })
            .unwrap_or(0);

        let price = main_model.map(|m| m.price).unwrap_or(0.0) / 100000.0;

        Product {
            id: shopee_product.product_id.to_string(),
            platform: "shopee".to_string(),
            name: shopee_product.product_name,
            description: shopee_product.description,
            category: shopee_product.category_id.map(|id| id.to_string()),
            sku: main_model.and_then(|m| m.model_id.to_string().into()),
            price,
            original_price: main_model.and_then(|m| m.original_price.map(|p| p / 100000.0)),
            stock: total_stock,
            images: shopee_product
                .image
                .and_then(|i| i.image_url_list)
                .unwrap_or_default(),
            status: Self::map_product_status(shopee_product.status),
            rating: shopee_product.rating_star,
            sold_count: shopee_product.historical_sold.unwrap_or(0),
            created_at: Self::timestamp_to_datetime(shopee_product.create_time),
            updated_at: Self::timestamp_to_datetime(shopee_product.update_time),
        }
    }

    async fn make_request<P, R>(&self, path: &str, params: P) -> anyhow::Result<R>
    where
        P: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .context("Failed to get timestamp")?
            .as_secs() as i64;

        let body = serde_json::to_string(&params)?;
        let signature = self.generate_signature(path, &body);

        let url = format!(
            "{}/api/v2/{}?partner_id={}&shopid={}&timestamp={}&sign={}",
            self.base_url,
            path.trim_start_matches('/'),
            self.config.partner_id,
            self.config.shop_id,
            timestamp,
            signature
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .context("Failed to make Shopee API request")?;

        let api_response: ShopeeResponse<R> = response
            .json()
            .await
            .context("Failed to parse Shopee response")?;

        if let Some(error) = api_response.error {
            anyhow::bail!(
                "Shopee API error: {} - {}",
                error,
                api_response.message.unwrap_or_default()
            );
        }

        api_response
            .response
            .context("Empty response from Shopee API")
    }
}

#[async_trait::async_trait]
impl EcommercePlatform for ShopeeApi {
    fn name(&self) -> &str {
        "shopee"
    }

    fn is_authenticated(&self) -> bool {
        !self.config.api_key.is_empty() && !self.config.secret_key.is_empty()
    }

    async fn authenticate(&mut self) -> anyhow::Result<()> {
        if self.is_authenticated() {
            tracing::info!("Shopee API credentials configured");
        } else {
            anyhow::bail!("Shopee API credentials not configured");
        }
        Ok(())
    }

    async fn get_orders(&self, status: Option<&str>) -> anyhow::Result<Vec<Order>> {
        let mut all_orders = Vec::new();
        let mut cursor = None;
        let mut has_more = true;

        while has_more {
            let request = OrderListRequest {
                order_status: status.map(String::from),
                page_size: Some(100),
                cursor,
                create_time_from: None,
                create_time_to: None,
            };

            let response: OrderListResponse =
                self.make_request("/order/get_order_list", request).await?;

            for shopee_order in response.order_list {
                all_orders.push(self.convert_order(shopee_order));
            }

            has_more = response.has_more;
            cursor = response.next_cursor;
        }

        Ok(all_orders)
    }

    async fn get_products(&self) -> anyhow::Result<Vec<Product>> {
        let mut all_products = Vec::new();
        let mut page_token = None;
        let mut has_more = true;

        while has_more {
            let request = ProductListRequest {
                page_size: Some(100),
                page_token,
                update_time_from: None,
                update_time_to: None,
                status: Some(1),
            };

            let response: ProductListResponse = self
                .make_request("/product/get_product_list", request)
                .await?;

            for shopee_product in response.product_list {
                all_products.push(self.convert_product(shopee_product));
            }

            has_more = response.has_more;
            page_token = response.next_page_token;
        }

        Ok(all_products)
    }

    async fn get_inventory(
        &self,
        product_ids: Option<Vec<String>>,
    ) -> anyhow::Result<Vec<InventoryItem>> {
        if product_ids.is_none() {
            return Ok(Vec::new());
        }

        let products = self.get_products().await?;
        let ids: Vec<i64> = products
            .iter()
            .filter(|p| {
                product_ids
                    .as_ref()
                    .map(|ids| ids.contains(&p.id))
                    .unwrap_or(false)
            })
            .map(|p| p.id.parse().unwrap_or(0))
            .collect();

        let mut inventory = Vec::new();

        for product_id in ids {
            let request = serde_json::json!({
                "item_id": product_id,
            });

            #[derive(Deserialize)]
            struct StockResponse {
                model: Option<Vec<ShopeeModel>>,
            }

            if let Ok(response) = self
                .make_request::<_, ShopeeResponse<StockResponse>>(
                    "/product/get_model_list",
                    request,
                )
                .await
            {
                if let Some(models) = response.response.and_then(|r| r.model) {
                    for model in models {
                        inventory.push(InventoryItem {
                            product_id: product_id.to_string(),
                            sku: Some(model.model_id.to_string()),
                            warehouse_id: None,
                            quantity: model
                                .stock_info
                                .as_ref()
                                .and_then(|s| s.first())
                                .and_then(|st| st.normal_stock)
                                .unwrap_or(0),
                            reserved_quantity: 0,
                            available_quantity: model
                                .stock_info
                                .as_ref()
                                .and_then(|s| s.first())
                                .and_then(|st| st.normal_stock)
                                .unwrap_or(0),
                            updated_at: Utc::now(),
                        });
                    }
                }
            }
        }

        Ok(inventory)
    }

    async fn update_inventory(&self, product_id: &str, quantity: i32) -> anyhow::Result<()> {
        let item_id: i64 = product_id.parse().context("Invalid product ID")?;

        let request = InventoryUpdateRequest {
            item_id,
            model_id: None,
            stock: quantity,
        };

        self.make_request::<_, serde_json::Value>("/product/update_stock", request)
            .await?;

        tracing::info!(
            "Updated Shopee inventory for product {} to {}",
            product_id,
            quantity
        );
        Ok(())
    }
}

impl ShopeeApi {
    pub async fn get_sales_report(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<SalesReport> {
        let start_ts = chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", start_date))
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|_| Utc::now().timestamp() - 86400 * 30);

        let end_ts = chrono::DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", end_date))
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|_| Utc::now().timestamp());

        let request = OrderListRequest {
            order_status: None,
            page_size: Some(100),
            cursor: None,
            create_time_from: Some(start_ts),
            create_time_to: Some(end_ts),
        };

        let response: OrderListResponse =
            self.make_request("/order/get_order_list", request).await?;

        let orders: Vec<Order> = response
            .order_list
            .into_iter()
            .map(|o| self.convert_order(o))
            .collect();

        let total_orders = orders.len() as i32;
        let total_revenue: f64 = orders.iter().map(|o| o.total_amount).sum();
        let total_shipping: f64 = orders.iter().map(|o| o.shipping_fee).sum();
        let total_discount: f64 = orders.iter().map(|o| o.discount).sum();

        let mut product_sales: std::collections::HashMap<String, (String, i32, f64)> =
            std::collections::HashMap::new();

        for order in &orders {
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

        for order in &orders {
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
            platform: "shopee".to_string(),
            period_start: chrono::DateTime::from_timestamp(start_ts, 0).unwrap_or_else(Utc::now),
            period_end: chrono::DateTime::from_timestamp(end_ts, 0).unwrap_or_else(Utc::now),
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

    fn create_test_config() -> ShopeeConfig {
        ShopeeConfig {
            partner_id: 123456,
            shop_id: 789012,
            api_key: "test_api_key".to_string(),
            secret_key: "test_secret_key".to_string(),
        }
    }

    #[test]
    fn test_signature_generation() {
        let config = create_test_config();
        let api = ShopeeApi::new(config);
        let signature = api.generate_signature("/api/v2/test", "{\"key\":\"value\"}");
        assert!(!signature.is_empty());
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(ShopeeApi::map_status("UNPAID"), OrderStatus::Pending);
        assert_eq!(ShopeeApi::map_status("SHIPPED"), OrderStatus::Shipped);
        assert_eq!(ShopeeApi::map_status("COMPLETED"), OrderStatus::Delivered);
    }

    #[tokio::test]
    async fn test_shopee_api_creation() {
        let config = create_test_config();
        let api = ShopeeApi::new(config);
        assert_eq!(api.name(), "shopee");
        assert!(api.is_authenticated());
    }
}
