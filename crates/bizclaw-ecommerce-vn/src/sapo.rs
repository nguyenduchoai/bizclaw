//! Sapo MCP Integration
//!
//! Connects to Sapo MCP server via stdio transport.

use crate::types::{Order, OrderStatus, Product, Customer, EcommercePlatform};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub struct SapoAdapter {
    store: String,
    api_key: String,
    api_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SapoConfig {
    pub store: String,
    pub api_key: String,
    pub api_secret: String,
}

impl SapoAdapter {
    pub fn new(store: &str, api_key: &str, api_secret: &str) -> Self {
        Self {
            store: store.to_string(),
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
        }
    }

    pub fn from_config(config: &SapoConfig) -> Self {
        Self::new(&config.store, &config.api_key, &config.api_secret)
    }

    pub fn get_command(&self) -> (&str, Vec<String>, std::collections::HashMap<String, String>) {
        let mut env = std::collections::HashMap::new();
        env.insert("SAPO_STORE".to_string(), self.store.clone());
        env.insert("SAPO_API_KEY".to_string(), self.api_key.clone());
        env.insert("SAPO_API_SECRET".to_string(), self.api_secret.clone());

        (
            "npx",
            vec![
                "-y".to_string(),
                "sapo-mcp@latest".to_string(),
                "--mode=pos-online,web,analytics".to_string(),
            ],
            env,
        )
    }

    pub fn get_tools(&self) -> Vec<&'static str> {
        vec![
            "list_orders",
            "get_order",
            "list_products",
            "get_product",
            "list_customers",
            "get_customer",
            "search_customers",
            "get_inventory_levels",
            "revenue_summary",
            "top_products",
            "customer_ltv",
        ]
    }

    pub fn parse_order(&self, data: &serde_json::Value) -> Result<Order> {
        let id = data["id"].as_i64().unwrap_or(0).to_string();
        let name = data["name"].as_str().unwrap_or(&id);
        let status_str = data["fulfillment_status"].as_str().unwrap_or("pending");

        let customer = Customer {
            id: data["customer"]["id"].as_i64().unwrap_or(0).to_string(),
            name: data["customer"]["first_name"]
                .as_str()
                .map(|f| format!("{} {}", f, data["customer"]["last_name"].as_str().unwrap_or("")))
                .unwrap_or_default(),
            email: data["customer"]["email"].as_str().map(String::from),
            phone: data["customer"]["phone"].as_str().map(String::from),
            address: None,
            total_orders: data["customer"]["orders_count"].as_i64().unwrap_or(0) as i32,
            total_spent: data["customer"]["total_spent"]
                .as_f64()
                .unwrap_or(0.0),
            last_order_at: None,
            tags: vec![],
            platform_customer_id: data["customer"]["id"].as_i64().unwrap_or(0).to_string(),
        };

        let line_items = data["line_items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| crate::types::OrderItem {
                        id: item["id"].as_i64().unwrap_or(0).to_string(),
                        product_id: item["product_id"].as_i64().unwrap_or(0).to_string(),
                        product_name: item["title"].as_str().unwrap_or("").to_string(),
                        sku: item["sku"].as_str().map(String::from),
                        variant_id: item["variant_id"].as_i64().map(|v| v.to_string()),
                        variant_name: item["variant_title"].as_str().map(String::from),
                        quantity: item["quantity"].as_i64().unwrap_or(0) as i32,
                        unit_price: item["price"].as_f64().unwrap_or(0.0),
                        discount: item["discount"]
                            .as_f64()
                            .unwrap_or(0.0),
                        total_price: item["price"]
                            .as_f64()
                            .unwrap_or(0.0)
                            * item["quantity"].as_f64().unwrap_or(1.0),
                        image_url: item["image"]
                            .as_str()
                            .map(String::from),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let subtotal = data["subtotal_price"].as_f64().unwrap_or(0.0);
        let total_shipping = data["total_shipping_price_set"]["shop_money"]["amount"]
            .as_f64()
            .unwrap_or(0.0);
        let total_discounts = data["total_discounts"].as_f64().unwrap_or(0.0);
        let total_price = data["total_price"].as_f64().unwrap_or(0.0);

        Ok(Order {
            id: id.clone(),
            platform: EcommercePlatform::Sapo,
            external_id: name.to_string(),
            status: OrderStatus::from_platform(status_str, "sapo"),
            customer,
            items: line_items,
            pricing: crate::types::OrderPricing {
                subtotal,
                shipping_fee: total_shipping,
                discount: total_discounts,
                tax: 0.0,
                total: total_price,
                currency: "VND".to_string(),
            },
            shipping: crate::types::ShippingInfo {
                carrier: data["shipping_lines"]
                    .as_array()
                    .and_then(|l| l.first())
                    .and_then(|l| l["title"].as_str())
                    .map(String::from),
                tracking_number: data["fulfillments"]
                    .as_array()
                    .and_then(|f| f.first())
                    .and_then(|f| f["tracking_number"].as_str())
                    .map(String::from),
                estimated_delivery: None,
                shipped_at: None,
            },
            fulfillment: crate::types::FulfillmentInfo {
                status: crate::types::FulfillmentStatus::Unfulfilled,
                itemsfulfilled: 0,
                total_items: 0,
            },
            payment: crate::types::PaymentInfo {
                method: data["payment_details"]["method"].as_str().map(String::from),
                status: crate::types::PaymentStatus::Pending,
                transaction_id: data["transaction_id"].as_str().map(String::from),
                paid_at: None,
            },
            created_at: chrono::DateTime::parse_from_rfc3339(
                data["created_at"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
            )
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::Utc::now(),
            notes: data["note"].as_str().map(String::from),
            source: data["source_name"].as_str().map(String::from),
        })
    }

    pub fn parse_product(&self, data: &serde_json::Value) -> Result<Product> {
        let id = data["id"].as_i64().unwrap_or(0).to_string();

        Ok(Product {
            id: id.clone(),
            platform: EcommercePlatform::Sapo,
            external_id: id,
            name: data["title"].as_str().unwrap_or("").to_string(),
            description: data["body_html"].as_str().map(String::from),
            category: data["product_type"].as_str().map(String::from),
            sku: data["variants"]
                .as_array()
                .and_then(|v| v.first())
                .and_then(|v| v["sku"].as_str())
                .map(String::from),
            variants: vec![],
            images: data["images"]
                .as_array()
                .map(|imgs| {
                    imgs.iter()
                        .filter_map(|img| img["src"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            price: crate::types::ProductPrice {
                price: data["variants"]
                    .as_array()
                    .and_then(|v| v.first())
                    .and_then(|v| v["price"].as_f64())
                    .unwrap_or(0.0),
                compare_at_price: data["variants"]
                    .as_array()
                    .and_then(|v| v.first())
                    .and_then(|v| v["compare_at_price"].as_f64()),
                cost_price: None,
                currency: "VND".to_string(),
            },
            inventory: crate::types::InventoryInfo {
                tracking: true,
                quantity: 0,
                available: 0,
                reserved: 0,
                locations: vec![],
            },
            status: crate::types::ProductStatus::Active,
            tags: data["tags"]
                .as_str()
                .map(|t| t.split(',').map(String::from).collect())
                .unwrap_or_default(),
            created_at: chrono::DateTime::parse_from_rfc3339(
                data["created_at"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
            )
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sapo_adapter_creation() {
        let adapter = SapoAdapter::new("mystore", "key", "secret");
        assert_eq!(adapter.store, "mystore");
    }

    #[test]
    fn test_get_command() {
        let adapter = SapoAdapter::new("mystore", "key", "secret");
        let (cmd, args, env) = adapter.get_command();

        assert_eq!(cmd, "npx");
        assert!(args.contains(&"sapo-mcp@latest".to_string()));
        assert_eq!(env.get("SAPO_STORE"), Some(&"mystore".to_string()));
    }
}
