//! Shared types for Vietnamese E-commerce platforms

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EcommercePlatform {
    Sapo,
    Haravan,
    KiotViet,
    LadiSales,
    #[serde(other)]
    Unknown,
}

impl EcommercePlatform {
    pub fn code(&self) -> &'static str {
        match self {
            EcommercePlatform::Sapo => "sapo",
            EcommercePlatform::Haravan => "haravan",
            EcommercePlatform::KiotViet => "kiotviet",
            EcommercePlatform::LadiSales => "ladisales",
            EcommercePlatform::Unknown => "unknown",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "sapo" => EcommercePlatform::Sapo,
            "haravan" => EcommercePlatform::Haravan,
            "kiotviet" | "kiotviet-mcp" => EcommercePlatform::KiotViet,
            "ladisales" | "ladipage" | "ladisales-mcp" => EcommercePlatform::LadiSales,
            _ => EcommercePlatform::Unknown,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            EcommercePlatform::Sapo => "Sapo POS & Online",
            EcommercePlatform::Haravan => "Haravan",
            EcommercePlatform::KiotViet => "KiotViet",
            EcommercePlatform::LadiSales => "LadiSales",
            EcommercePlatform::Unknown => "Unknown",
        }
    }

    pub fn api_docs_url(&self) -> &'static str {
        match self {
            EcommercePlatform::Sapo => "https://api.sapo.vn/",
            EcommercePlatform::Haravan => "https://docs.haravan.com/docs/openapi",
            EcommercePlatform::KiotViet => "https://docs.kiotviet.vn/",
            EcommercePlatform::LadiSales => "https://docs.sales.ldpform.net/",
            EcommercePlatform::Unknown => "",
        }
    }
}

impl std::fmt::Display for EcommercePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub platform: EcommercePlatform,
    pub external_id: String,
    pub status: OrderStatus,
    pub customer: Customer,
    pub items: Vec<OrderItem>,
    pub pricing: OrderPricing,
    pub shipping: ShippingInfo,
    pub fulfillment: FulfillmentInfo,
    pub payment: PaymentInfo,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub notes: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<Address>,
    pub total_orders: i32,
    pub total_spent: f64,
    pub last_order_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub platform_customer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub ward: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

impl Address {
    pub fn full_address(&self) -> String {
        let parts: Vec<&str> = [
            self.street.as_deref(),
            self.ward.as_deref(),
            self.district.as_deref(),
            self.city.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();

        parts.join(", ")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub sku: Option<String>,
    pub variant_id: Option<String>,
    pub variant_name: Option<String>,
    pub quantity: i32,
    pub unit_price: f64,
    pub discount: f64,
    pub total_price: f64,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPricing {
    pub subtotal: f64,
    pub shipping_fee: f64,
    pub discount: f64,
    pub tax: f64,
    pub total: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingInfo {
    pub carrier: Option<String>,
    pub tracking_number: Option<String>,
    pub estimated_delivery: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentInfo {
    pub status: FulfillmentStatus,
    pub itemsfulfilled: i32,
    pub total_items: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FulfillmentStatus {
    Unfulfilled,
    PartiallyFulfilled,
    Fulfilled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl OrderStatus {
    pub fn from_platform(status: &str, platform: &str) -> Self {
        let status_lower = status.to_lowercase();

        match platform.to_lowercase().as_str() {
            "sapo" => match status_lower.as_str() {
                "pending" | "chờ xử lý" => OrderStatus::Pending,
                "confirmed" | "đã xác nhận" => OrderStatus::Confirmed,
                "processing" | "đang xử lý" => OrderStatus::Processing,
                "shipped" | "đã giao cho đơn vị vận chuyển" => OrderStatus::Shipped,
                "delivered" | "hoàn thành" => OrderStatus::Delivered,
                "cancelled" | "đã hủy" => OrderStatus::Cancelled,
                "returned" | "trả hàng" => OrderStatus::Returned,
                "refunded" | "đã hoàn tiền" => OrderStatus::Refunded,
                _ => OrderStatus::Unknown,
            },
            "haravan" => match status_lower.as_str() {
                "open" | "pending" => OrderStatus::Pending,
                "confirmed" => OrderStatus::Confirmed,
                "processing" | "inprogress" => OrderStatus::Processing,
                "shipped" | "fulfilled" => OrderStatus::Shipped,
                "closed" | "completed" => OrderStatus::Delivered,
                "cancelled" => OrderStatus::Cancelled,
                "voided" => OrderStatus::Cancelled,
                _ => OrderStatus::Unknown,
            },
            "kiotviet" => match status_lower.as_str() {
                "pending" | "new" | "initialized" => OrderStatus::Pending,
                "confirmed" | "approved" => OrderStatus::Confirmed,
                "processing" | "picking" => OrderStatus::Processing,
                "shipped" | "delivering" => OrderStatus::Shipped,
                "completed" | "delivered" => OrderStatus::Delivered,
                "cancelled" | "canceled" => OrderStatus::Cancelled,
                "return" | "returned" => OrderStatus::Returned,
                _ => OrderStatus::Unknown,
            },
            "ladisales" => match status_lower.as_str() {
                "pending" | "chờ xử lý" => OrderStatus::Pending,
                "confirmed" | "đã xác nhận" => OrderStatus::Confirmed,
                "processing" | "đang xử lý" => OrderStatus::Processing,
                "shipped" | "đã giao" => OrderStatus::Shipped,
                "delivered" | "hoàn thành" => OrderStatus::Delivered,
                "cancelled" | "đã hủy" => OrderStatus::Cancelled,
                "returned" | "trả hàng" => OrderStatus::Returned,
                _ => OrderStatus::Unknown,
            },
            _ => OrderStatus::Unknown,
        }
    }

    pub fn to_sapo(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Confirmed => "confirmed",
            OrderStatus::Processing => "processing",
            OrderStatus::Shipped => "shipped",
            OrderStatus::Delivered => "delivered",
            OrderStatus::Cancelled => "cancelled",
            OrderStatus::Returned => "returned",
            OrderStatus::Refunded => "refunded",
            OrderStatus::Unknown => "unknown",
        }
    }

    pub fn to_haravan(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "open",
            OrderStatus::Confirmed => "confirmed",
            OrderStatus::Processing => "inprogress",
            OrderStatus::Shipped => "shipped",
            OrderStatus::Delivered => "closed",
            OrderStatus::Cancelled => "cancelled",
            OrderStatus::Returned => "returned",
            OrderStatus::Refunded => "refunded",
            OrderStatus::Unknown => "unknown",
        }
    }

    pub fn to_kiotviet(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "Pending",
            OrderStatus::Confirmed => "Confirmed",
            OrderStatus::Processing => "Processing",
            OrderStatus::Shipped => "Shipped",
            OrderStatus::Delivered => "Completed",
            OrderStatus::Cancelled => "Cancelled",
            OrderStatus::Returned => "Return",
            OrderStatus::Refunded => "Refunded",
            OrderStatus::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInfo {
    pub method: Option<String>,
    pub status: PaymentStatus,
    pub transaction_id: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Paid,
    PartiallyPaid,
    Refunded,
    Voided,
    Unknown,
}

impl PaymentStatus {
    pub fn from_platform(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "pending" | "chưa thanh toán" | "unpaid" => PaymentStatus::Pending,
            "paid" | "đã thanh toán" | "completed" => PaymentStatus::Paid,
            "partially_paid" | "thanh toán một phần" => PaymentStatus::PartiallyPaid,
            "refunded" | "đã hoàn tiền" => PaymentStatus::Refunded,
            "voided" | "đã hủy" => PaymentStatus::Voided,
            _ => PaymentStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub platform: EcommercePlatform,
    pub external_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub sku: Option<String>,
    pub variants: Vec<ProductVariant>,
    pub images: Vec<String>,
    pub price: ProductPrice,
    pub inventory: InventoryInfo,
    pub status: ProductStatus,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub id: String,
    pub external_id: String,
    pub sku: Option<String>,
    pub name: Option<String>,
    pub price: f64,
    pub compare_at_price: Option<f64>,
    pub inventory_quantity: i32,
    pub image_url: Option<String>,
    pub options: Vec<ProductOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductOption {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPrice {
    pub price: f64,
    pub compare_at_price: Option<f64>,
    pub cost_price: Option<f64>,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryInfo {
    pub tracking: bool,
    pub quantity: i32,
    pub available: i32,
    pub reserved: i32,
    pub locations: Vec<InventoryLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryLocation {
    pub location_id: String,
    pub location_name: String,
    pub quantity: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    Active,
    Inactive,
    Draft,
    Archived,
    Unknown,
}

impl ProductStatus {
    pub fn from_platform(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "active" | "đang bán" | "published" => ProductStatus::Active,
            "inactive" | "ngừng bán" | "unpublished" => ProductStatus::Inactive,
            "draft" | "nháp" => ProductStatus::Draft,
            "archived" | "lưu trữ" => ProductStatus::Archived,
            _ => ProductStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryLevel {
    pub product_id: String,
    pub variant_id: Option<String>,
    pub location_id: String,
    pub location_name: String,
    pub quantity: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    pub platform: EcommercePlatform,
    pub event_type: WebhookEventType,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub processed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    OrderCreated,
    OrderUpdated,
    OrderCancelled,
    OrderFulfilled,
    OrderPaid,
    ProductCreated,
    ProductUpdated,
    ProductDeleted,
    InventoryUpdated,
    CustomerCreated,
    CustomerUpdated,
    Unknown,
}

impl WebhookEventType {
    pub fn from_string(event: &str, platform: &str) -> Self {
        let event_lower = event.to_lowercase();

        match platform.to_lowercase().as_str() {
            "sapo" => match event_lower.as_str() {
                "orders/create" | "order_created" => WebhookEventType::OrderCreated,
                "orders/update" | "order_updated" => WebhookEventType::OrderUpdated,
                "orders/cancelled" | "order_cancelled" => WebhookEventType::OrderCancelled,
                "orders/fulfilled" | "order_fulfilled" => WebhookEventType::OrderFulfilled,
                "orders/paid" | "order_paid" => WebhookEventType::OrderPaid,
                "products/create" | "product_created" => WebhookEventType::ProductCreated,
                "products/update" | "product_updated" => WebhookEventType::ProductUpdated,
                "products/delete" | "product_deleted" => WebhookEventType::ProductDeleted,
                "inventory_levels/update" => WebhookEventType::InventoryUpdated,
                "customers/create" => WebhookEventType::CustomerCreated,
                "customers/update" => WebhookEventType::CustomerUpdated,
                _ => WebhookEventType::Unknown,
            },
            "haravan" => match event_lower.as_str() {
                "orders/create" => WebhookEventType::OrderCreated,
                "orders/update" => WebhookEventType::OrderUpdated,
                "orders/cancelled" => WebhookEventType::OrderCancelled,
                "orders/fulfilled" => WebhookEventType::OrderFulfilled,
                "orders/paid" => WebhookEventType::OrderPaid,
                "products/create" => WebhookEventType::ProductCreated,
                "products/update" => WebhookEventType::ProductUpdated,
                "products/delete" => WebhookEventType::ProductDeleted,
                _ => WebhookEventType::Unknown,
            },
            _ => WebhookEventType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}
