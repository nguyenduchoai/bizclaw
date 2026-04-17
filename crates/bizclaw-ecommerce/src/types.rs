//! Shared types for e-commerce platforms

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub platform: String,
    pub status: OrderStatus,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub customer_address: Option<String>,
    pub total_amount: f64,
    pub shipping_fee: f64,
    pub discount: f64,
    pub items: Vec<OrderItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
    Returned,
    Refunded,
    Unknown,
}

impl From<&str> for OrderStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" | "chờ_xử_lý" | "等待处理" => OrderStatus::Pending,
            "confirmed" | "đã_xác_nhận" | "已确认" => OrderStatus::Confirmed,
            "processing" | "đang_xử_lý" | "处理中" => OrderStatus::Processing,
            "shipped" | "đã_giao" | "已发货" => OrderStatus::Shipped,
            "delivered" | "hoàn_thành" | "已完成" => OrderStatus::Delivered,
            "cancelled" | "đã_hủy" | "已取消" => OrderStatus::Cancelled,
            "returned" | "trả_hàng" | "已退货" => OrderStatus::Returned,
            "refunded" | "hoàn_tiền" | "已退款" => OrderStatus::Refunded,
            _ => OrderStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub sku: Option<String>,
    pub quantity: i32,
    pub unit_price: f64,
    pub discount: f64,
    pub total_price: f64,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub platform: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub sku: Option<String>,
    pub price: f64,
    pub original_price: Option<f64>,
    pub stock: i32,
    pub images: Vec<String>,
    pub status: ProductStatus,
    pub rating: Option<f32>,
    pub sold_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    Active,
    Inactive,
    Deleted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub product_id: String,
    pub sku: Option<String>,
    pub warehouse_id: Option<String>,
    pub quantity: i32,
    pub reserved_quantity: i32,
    pub available_quantity: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesReport {
    pub platform: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_orders: i32,
    pub total_revenue: f64,
    pub total_shipping_fee: f64,
    pub total_discount: f64,
    pub total_refunds: f64,
    pub net_revenue: f64,
    pub top_products: Vec<TopProduct>,
    pub daily_breakdown: Vec<DailySales>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProduct {
    pub product_id: String,
    pub product_name: String,
    pub quantity_sold: i32,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySales {
    pub date: String,
    pub orders: i32,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event_type: String,
    pub platform: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_parsing() {
        assert_eq!(OrderStatus::from("pending"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from("shipped"), OrderStatus::Shipped);
        assert_eq!(OrderStatus::from("delivered"), OrderStatus::Delivered);
        assert_eq!(OrderStatus::from("unknown_status"), OrderStatus::Unknown);
    }

    #[test]
    fn test_order_serialization() {
        let order = Order {
            id: "ORD123".to_string(),
            platform: "tiktok".to_string(),
            status: OrderStatus::Pending,
            customer_name: "Nguyễn Văn A".to_string(),
            customer_phone: Some("0912345678".to_string()),
            customer_address: Some("123 Đường ABC, TP.HCM".to_string()),
            total_amount: 500000.0,
            shipping_fee: 30000.0,
            discount: 0.0,
            items: vec![OrderItem {
                id: "ITEM1".to_string(),
                product_id: "PROD1".to_string(),
                product_name: "Sản phẩm A".to_string(),
                sku: Some("SKU001".to_string()),
                quantity: 2,
                unit_price: 250000.0,
                discount: 0.0,
                total_price: 500000.0,
                image_url: None,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            notes: None,
        };

        let json = serde_json::to_string(&order).unwrap();
        let parsed: Order = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "ORD123");
    }
}
