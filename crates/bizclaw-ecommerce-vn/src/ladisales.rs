//! LadiSales (LadiPage) MCP Integration
//!
//! Connects to LadiSales MCP server.

use crate::types::{Order, OrderStatus, Product, Customer, EcommercePlatform};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct LadiSalesAdapter {
    api_key: String,
    store_id: String,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LadiSalesConfig {
    pub api_key: String,
    pub store_id: String,
    #[serde(default = "default_ladisales_url")]
    pub base_url: String,
}

fn default_ladisales_url() -> String {
    "https://apiv5.sales.ldpform.net/2.0".to_string()
}

impl LadiSalesAdapter {
    pub fn new(api_key: &str, store_id: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            store_id: store_id.to_string(),
            base_url: default_ladisales_url(),
        }
    }

    pub fn from_config(config: &LadiSalesConfig) -> Self {
        Self {
            api_key: config.api_key.clone(),
            store_id: config.store_id.clone(),
            base_url: config.base_url.clone(),
        }
    }

    pub fn get_command(&self) -> (&str, Vec<String>, std::collections::HashMap<String, String>) {
        let mut env = std::collections::HashMap::new();
        env.insert("LADISALES_API_KEY".to_string(), self.api_key.clone());
        env.insert("LADISALES_STORE_ID".to_string(), self.store_id.clone());

        (
            "npx",
            vec![
                "-y".to_string(),
                "ladipage-mcp".to_string(),
            ],
            env,
        )
    }

    pub fn get_tools(&self) -> Vec<&'static str> {
        vec![
            "manage_products_list",
            "manage_products_show",
            "manage_products_create",
            "manage_products_update",
            "manage_products_delete",
            "manage_orders_list",
            "manage_orders_show",
            "manage_orders_create",
            "manage_orders_cancel",
            "manage_customers_show",
            "manage_customers_create",
            "manage_customers_update",
            "manage_discounts_create",
            "manage_shipping_list_methods",
        ]
    }

    pub fn parse_order(&self, data: &serde_json::Value) -> Result<Order> {
        let id = data["id"].as_str().unwrap_or("0").to_string();
        let code = data["order_code"].as_str().unwrap_or(&id);

        let status_str = data["status"].as_str().unwrap_or("pending");
        let status = OrderStatus::from_platform(status_str, "ladisales");

        let customer_data = &data["customer"];
        let customer = Customer {
            id: customer_data["id"].as_str().unwrap_or("0").to_string(),
            name: customer_data["name"].as_str().unwrap_or("").to_string(),
            email: customer_data["email"].as_str().map(String::from),
            phone: customer_data["phone"].as_str().map(String::from),
            address: None,
            total_orders: 1,
            total_spent: data["total"].as_f64().unwrap_or(0.0),
            last_order_at: None,
            tags: vec![],
            platform_customer_id: customer_data["id"].as_str().unwrap_or("0").to_string(),
        };

        let line_items = data["line_items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| crate::types::OrderItem {
                        id: item["id"].as_str().unwrap_or("0").to_string(),
                        product_id: item["product_id"].as_str().unwrap_or("0").to_string(),
                        product_name: item["product_name"].as_str().unwrap_or("").to_string(),
                        sku: item["sku"].as_str().map(String::from),
                        variant_id: item["variant_id"].as_str().map(String::from),
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

        let total = data["total"].as_f64().unwrap_or(0.0);
        let discount = data["discount"].as_f64().unwrap_or(0.0);
        let shipping = data["shipping_fee"].as_f64().unwrap_or(0.0);

        Ok(Order {
            id: id.clone(),
            platform: EcommercePlatform::LadiSales,
            external_id: code.to_string(),
            status,
            customer,
            items: line_items,
            pricing: crate::types::OrderPricing {
                subtotal: total + discount - shipping,
                shipping_fee: shipping,
                discount,
                tax: 0.0,
                total,
                currency: "VND".to_string(),
            },
            shipping: crate::types::ShippingInfo {
                carrier: None,
                tracking_number: data["tracking_number"].as_str().map(String::from),
                estimated_delivery: None,
                shipped_at: None,
            },
            fulfillment: crate::types::FulfillmentInfo {
                status: crate::types::FulfillmentStatus::Unfulfilled,
                itemsfulfilled: 0,
                total_items: 0,
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
            source: Some("ladisales".to_string()),
        })
    }

    pub fn parse_product(&self, data: &serde_json::Value) -> Result<Product> {
        let id = data["id"].as_str().unwrap_or("0").to_string();

        Ok(Product {
            id: id.clone(),
            platform: EcommercePlatform::LadiSales,
            external_id: data["sku"].as_str().unwrap_or(&id).to_string(),
            name: data["name"].as_str().unwrap_or("").to_string(),
            description: data["description"].as_str().map(String::from),
            category: None,
            sku: data["sku"].as_str().map(String::from),
            variants: vec![],
            images: data["images"]
                .as_array()
                .map(|imgs| {
                    imgs.iter()
                        .filter_map(|img| img.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            price: crate::types::ProductPrice {
                price: data["price"].as_f64().unwrap_or(0.0),
                compare_at_price: data["compare_at_price"].as_f64(),
                cost_price: None,
                currency: "VND".to_string(),
            },
            inventory: crate::types::InventoryInfo {
                tracking: true,
                quantity: data["quantity"].as_i64().unwrap_or(0) as i32,
                available: data["quantity"].as_i64().unwrap_or(0) as i32,
                reserved: 0,
                locations: vec![],
            },
            status: if data["status"].as_str() == Some("active") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ladisales_adapter_creation() {
        let adapter = LadiSalesAdapter::new("api_key", "store_id");
        assert_eq!(adapter.api_key, "api_key");
        assert_eq!(adapter.store_id, "store_id");
    }

    #[test]
    fn test_get_command() {
        let adapter = LadiSalesAdapter::new("api_key", "store_id");
        let (cmd, args, env) = adapter.get_command();

        assert_eq!(cmd, "npx");
        assert!(args.contains(&"ladipage-mcp".to_string()));
        assert_eq!(env.get("LADISALES_API_KEY"), Some(&"api_key".to_string()));
        assert_eq!(env.get("LADISALES_STORE_ID"), Some(&"store_id".to_string()));
    }
}
