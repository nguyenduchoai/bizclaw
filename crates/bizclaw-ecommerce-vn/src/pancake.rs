//! Pancake POS MCP Integration
//!
//! Connects to Pancake POS MCP server - Vietnamese POS with CRM, Multi-channel support.

use crate::types::{Customer, EcommercePlatform, Order, OrderStatus, Product};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct PancakeAdapter {
    api_key: String,
    shop_id: String,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PancakeConfig {
    pub api_key: String,
    pub shop_id: String,
    #[serde(default = "default_pancake_url")]
    pub base_url: String,
}

fn default_pancake_url() -> String {
    "https://pos.pages.fm/api/v1".to_string()
}

impl PancakeAdapter {
    pub fn new(api_key: &str, shop_id: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            shop_id: shop_id.to_string(),
            base_url: default_pancake_url(),
        }
    }

    pub fn from_config(config: &PancakeConfig) -> Self {
        Self {
            api_key: config.api_key.clone(),
            shop_id: config.shop_id.clone(),
            base_url: config.base_url.clone(),
        }
    }

    pub fn get_command(&self) -> (&str, Vec<String>, std::collections::HashMap<String, String>) {
        let mut env = std::collections::HashMap::new();
        env.insert("PANCAKE_POS_API_KEY".to_string(), self.api_key.clone());
        env.insert("PANCAKE_POS_SHOP_ID".to_string(), self.shop_id.clone());

        (
            "bun",
            vec![
                "run".to_string(),
                "pancake-pos-mcp/src/index.ts".to_string(),
            ],
            env,
        )
    }

    pub fn get_tools(&self) -> Vec<&'static str> {
        vec![
            // Core POS
            "manage_orders",
            "manage_products",
            "manage_customers",
            "manage_inventory",
            // Supply Chain
            "manage_warehouses",
            "manage_suppliers",
            "manage_purchases",
            "manage_transfers",
            "manage_stocktaking",
            // Sales
            "manage_returns",
            "manage_combos",
            "manage_promotions",
            "manage_vouchers",
            // CRM
            "manage_crm_contacts",
            "manage_crm_deals",
            "manage_crm_activities",
            // Multi-Channel
            "manage_ecommerce",
            "manage_livestream",
            // Operations
            "manage_employees",
            "manage_webhooks",
            "get_statistics",
            "get_shop_info",
            "lookup_address",
        ]
    }

    pub fn get_tool_categories(&self) -> Vec<ToolCategory> {
        vec![
            ToolCategory {
                name: "Core POS".to_string(),
                tools: vec![
                    "manage_orders",
                    "manage_products",
                    "manage_customers",
                    "manage_inventory",
                ],
            },
            ToolCategory {
                name: "Supply Chain".to_string(),
                tools: vec![
                    "manage_warehouses",
                    "manage_suppliers",
                    "manage_purchases",
                    "manage_transfers",
                    "manage_stocktaking",
                ],
            },
            ToolCategory {
                name: "Sales".to_string(),
                tools: vec![
                    "manage_returns",
                    "manage_combos",
                    "manage_promotions",
                    "manage_vouchers",
                ],
            },
            ToolCategory {
                name: "CRM".to_string(),
                tools: vec![
                    "manage_crm_contacts",
                    "manage_crm_deals",
                    "manage_crm_activities",
                ],
            },
            ToolCategory {
                name: "Multi-Channel".to_string(),
                tools: vec!["manage_ecommerce", "manage_livestream"],
            },
            ToolCategory {
                name: "Operations".to_string(),
                tools: vec![
                    "manage_employees",
                    "manage_webhooks",
                    "get_statistics",
                    "get_shop_info",
                    "lookup_address",
                ],
            },
        ]
    }

    pub fn parse_order(&self, data: &serde_json::Value) -> Result<Order> {
        let id = data["id"].as_i64().unwrap_or(0).to_string();
        let code = data["code"].as_str().unwrap_or(&id);

        let status_str = data["status"].as_str().unwrap_or("pending");
        let status = self.parse_order_status(status_str);

        let customer_data = &data["customer"];
        let customer = Customer {
            id: customer_data["id"].as_i64().unwrap_or(0).to_string(),
            name: customer_data["name"].as_str().unwrap_or("").to_string(),
            email: customer_data["email"].as_str().map(String::from),
            phone: customer_data["phone"].as_str().map(String::from),
            address: None,
            total_orders: 1,
            total_spent: data["total"].as_f64().unwrap_or(0.0),
            last_order_at: None,
            tags: vec![],
            platform_customer_id: customer_data["id"].as_i64().unwrap_or(0).to_string(),
        };

        let items: Vec<crate::types::OrderItem> = data["items"]
            .as_array()
            .map(|item_array| {
                item_array
                    .iter()
                    .map(|item| crate::types::OrderItem {
                        id: item["id"].as_i64().unwrap_or(0).to_string(),
                        product_id: item["product_id"].as_i64().unwrap_or(0).to_string(),
                        product_name: item["product_name"].as_str().unwrap_or("").to_string(),
                        sku: item["sku"].as_str().map(String::from),
                        variant_id: item["variant_id"].as_i64().map(|v| v.to_string()),
                        variant_name: item["variant_name"].as_str().map(String::from),
                        quantity: item["quantity"].as_i64().unwrap_or(0) as i32,
                        unit_price: item["price"].as_f64().unwrap_or(0.0),
                        discount: item["discount"].as_f64().unwrap_or(0.0),
                        total_price: item["total"].as_f64().unwrap_or(0.0),
                        image_url: None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let items_count = items.len() as i32;

        let total = data["total"].as_f64().unwrap_or(0.0);
        let discount = data["discount"].as_f64().unwrap_or(0.0);
        let shipping = data["shipping_fee"].as_f64().unwrap_or(0.0);

        Ok(Order {
            id: id.clone(),
            platform: EcommercePlatform::Unknown,
            external_id: code.to_string(),
            status,
            customer,
            items,
            pricing: crate::types::OrderPricing {
                subtotal: total + discount - shipping,
                shipping_fee: shipping,
                discount,
                tax: 0.0,
                total,
                currency: "VND".to_string(),
            },
            shipping: crate::types::ShippingInfo {
                carrier: data["shipping_partner"].as_str().map(String::from),
                tracking_number: data["tracking_number"].as_str().map(String::from),
                estimated_delivery: None,
                shipped_at: None,
            },
            fulfillment: crate::types::FulfillmentInfo {
                status: crate::types::FulfillmentStatus::Unfulfilled,
                itemsfulfilled: 0,
                total_items: items_count,
            },
            payment: crate::types::PaymentInfo {
                method: data["payment_method"].as_str().map(String::from),
                status: crate::types::PaymentStatus::Pending,
                transaction_id: None,
                paid_at: None,
            },
            created_at: chrono::DateTime::parse_from_rfc3339(
                data["created_at"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
            )
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::Utc::now(),
            notes: data["note"].as_str().map(String::from),
            source: data["source"].as_str().map(String::from),
        })
    }

    fn parse_order_status(&self, status: &str) -> OrderStatus {
        match status.to_lowercase().as_str() {
            "pending" | "chờ xử lý" => OrderStatus::Pending,
            "confirmed" | "đã xác nhận" => OrderStatus::Confirmed,
            "processing" | "đang xử lý" => OrderStatus::Processing,
            "shipped" | "đang giao" => OrderStatus::Shipped,
            "delivered" | "hoàn thành" | "đã giao" => OrderStatus::Delivered,
            "cancelled" | "hủy" => OrderStatus::Cancelled,
            "returned" | "trả hàng" => OrderStatus::Returned,
            "refunded" | "đã hoàn tiền" => OrderStatus::Refunded,
            _ => OrderStatus::Unknown,
        }
    }

    pub fn parse_product(&self, data: &serde_json::Value) -> Result<Product> {
        let id = data["id"].as_i64().unwrap_or(0).to_string();

        let images = data["images"]
            .as_array()
            .map(|imgs| {
                imgs.iter()
                    .filter_map(|img| img["url"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let inventory_quantity = data["stock_quantity"].as_i64().unwrap_or(0) as i32;

        Ok(Product {
            id: id.clone(),
            platform: EcommercePlatform::Unknown,
            external_id: data["sku"].as_str().unwrap_or(&id).to_string(),
            name: data["name"].as_str().unwrap_or("").to_string(),
            description: data["description"].as_str().map(String::from),
            category: data["category"].as_str().map(String::from),
            sku: data["sku"].as_str().map(String::from),
            variants: vec![],
            images,
            price: crate::types::ProductPrice {
                price: data["price"].as_f64().unwrap_or(0.0),
                compare_at_price: data["compare_at_price"].as_f64(),
                cost_price: data["cost_price"].as_f64(),
                currency: "VND".to_string(),
            },
            inventory: crate::types::InventoryInfo {
                tracking: true,
                quantity: inventory_quantity,
                available: inventory_quantity,
                reserved: 0,
                locations: vec![],
            },
            status: if data["is_active"].as_bool().unwrap_or(true) {
                crate::types::ProductStatus::Active
            } else {
                crate::types::ProductStatus::Inactive
            },
            tags: data["tags"]
                .as_array()
                .map(|tags| {
                    tags.iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ToolCategory {
    pub name: String,
    pub tools: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderQuery {
    pub status: Option<String>,
    pub source: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductQuery {
    pub category_id: Option<String>,
    pub is_active: Option<bool>,
    pub keyword: Option<String>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerQuery {
    pub keyword: Option<String>,
    pub customer_type: Option<String>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsQuery {
    pub stat_type: String,
    pub from_date: String,
    pub to_date: String,
    pub group_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcommerceSync {
    pub platform: String,
    pub sync_type: String,
    pub entities: Vec<String>,
}

impl EcommerceSync {
    pub fn new(platform: &str, sync_type: &str) -> Self {
        Self {
            platform: platform.to_string(),
            sync_type: sync_type.to_string(),
            entities: vec!["products".to_string(), "orders".to_string(), "inventory".to_string()],
        }
    }

    pub fn products_only(platform: &str) -> Self {
        Self {
            platform: platform.to_string(),
            sync_type: "products".to_string(),
            entities: vec!["products".to_string()],
        }
    }

    pub fn orders_only(platform: &str) -> Self {
        Self {
            platform: platform.to_string(),
            sync_type: "orders".to_string(),
            entities: vec!["orders".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrmContact {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub position: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrmDeal {
    pub id: String,
    pub title: String,
    pub value: f64,
    pub stage: String,
    pub contact_id: Option<String>,
    pub expected_close_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrmActivity {
    pub id: String,
    pub activity_type: String,
    pub title: String,
    pub contact_id: Option<String>,
    pub deal_id: Option<String>,
    pub scheduled_at: Option<String>,
    pub completed_at: Option<String>,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pancake_adapter_creation() {
        let adapter = PancakeAdapter::new("test_api_key", "123456");
        assert_eq!(adapter.api_key, "test_api_key");
        assert_eq!(adapter.shop_id, "123456");
    }

    #[test]
    fn test_get_command() {
        let adapter = PancakeAdapter::new("test_api_key", "123456");
        let (cmd, args, env) = adapter.get_command();

        assert_eq!(cmd, "bun");
        assert!(args.contains(&"pancake-pos-mcp/src/index.ts".to_string()));
        assert_eq!(env.get("PANCAKE_POS_API_KEY"), Some(&"test_api_key".to_string()));
        assert_eq!(env.get("PANCAKE_POS_SHOP_ID"), Some(&"123456".to_string()));
    }

    #[test]
    fn test_tool_categories() {
        let adapter = PancakeAdapter::new("test", "123");
        let categories = adapter.get_tool_categories();

        assert!(categories.iter().any(|c| c.name == "Core POS"));
        assert!(categories.iter().any(|c| c.name == "Supply Chain"));
        assert!(categories.iter().any(|c| c.name == "CRM"));
        assert!(categories.iter().any(|c| c.name == "Multi-Channel"));
    }

    #[test]
    fn test_ecommerce_sync() {
        let sync = EcommerceSync::new("shopee", "full");
        assert_eq!(sync.platform, "shopee");
        assert_eq!(sync.sync_type, "full");
        assert!(sync.entities.contains(&"products".to_string()));

        let products_only = EcommerceSync::products_only("lazada");
        assert_eq!(products_only.entities, vec!["products".to_string()]);
    }

    #[test]
    fn test_order_status_parsing() {
        let adapter = PancakeAdapter::new("test", "123");

        assert_eq!(adapter.parse_order_status("pending"), OrderStatus::Pending);
        assert_eq!(adapter.parse_order_status("confirmed"), OrderStatus::Confirmed);
        assert_eq!(adapter.parse_order_status("shipped"), OrderStatus::Shipped);
        assert_eq!(adapter.parse_order_status("delivered"), OrderStatus::Delivered);
        assert_eq!(adapter.parse_order_status("cancelled"), OrderStatus::Cancelled);
    }
}
