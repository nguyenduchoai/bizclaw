//! KiotViet MCP Integration
//!
//! Connects to KiotViet MCP server with auto token refresh support.

use crate::types::{Order, OrderStatus, Product, Customer, EcommercePlatform};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct KiotVietAdapter {
    client_id: String,
    client_secret: String,
    retailer: String,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KiotVietConfig {
    pub client_id: String,
    pub client_secret: String,
    pub retailer: String,
    #[serde(default = "default_kiotviet_url")]
    pub base_url: String,
}

fn default_kiotviet_url() -> String {
    "https://public.kiotapi.com".to_string()
}

impl KiotVietAdapter {
    pub fn new(client_id: &str, client_secret: &str, retailer: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            retailer: retailer.to_string(),
            base_url: default_kiotviet_url(),
        }
    }

    pub fn from_config(config: &KiotVietConfig) -> Self {
        Self {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            retailer: config.retailer.clone(),
            base_url: config.base_url.clone(),
        }
    }

    pub fn get_command(&self) -> (&str, Vec<String>, std::collections::HashMap<String, String>) {
        let mut env = std::collections::HashMap::new();
        env.insert(
            "KIOTVIET_CLIENT_ID".to_string(),
            self.client_id.clone(),
        );
        env.insert(
            "KIOTVIET_CLIENT_SECRET".to_string(),
            self.client_secret.clone(),
        );
        env.insert("KIOTVIET_RETAILER".to_string(), self.retailer.clone());

        (
            "npx",
            vec![
                "-y".to_string(),
                "kiotviet-mcp".to_string(),
            ],
            env,
        )
    }

    pub fn get_tools(&self) -> Vec<&'static str> {
        vec![
            "kiotviet_products_list",
            "kiotviet_products_get_by_id",
            "kiotviet_products_create",
            "kiotviet_products_update",
            "kiotviet_orders_list",
            "kiotviet_orders_get",
            "kiotviet_orders_create",
            "kiotviet_orders_cancel",
            "kiotviet_customers_list",
            "kiotviet_customers_get",
            "kiotviet_invoices_list",
            "kiotviet_branches_list",
            "kiotviet_suppliers_list",
        ]
    }

    pub fn get_presets(&self) -> Vec<&'static str> {
        vec!["preset.default", "preset.readonly", "preset.products", "preset.sales", "preset.inventory"]
    }

    pub fn parse_order(&self, data: &serde_json::Value) -> Result<Order> {
        let id = data["id"].as_i64().unwrap_or(0).to_string();
        let code = data["code"].as_str().unwrap_or(&id);

        let status_str = data["status"].as_i64().unwrap_or(0);
        let status = match status_str {
            1 => OrderStatus::Pending,
            2 => OrderStatus::Confirmed,
            3 => OrderStatus::Processing,
            4 => OrderStatus::Shipped,
            5 => OrderStatus::Delivered,
            6 => OrderStatus::Cancelled,
            7 => OrderStatus::Returned,
            _ => OrderStatus::Unknown,
        };

        let customer_data = &data["customer"];
        let customer = Customer {
            id: customer_data["id"].as_i64().unwrap_or(0).to_string(),
            name: customer_data["name"].as_str().unwrap_or("").to_string(),
            email: customer_data["email"].as_str().map(String::from),
            phone: customer_data["mobile"].as_str().map(String::from),
            address: None,
            total_orders: data["orderDetails"]
                .as_array()
                .map(|_| 1)
                .unwrap_or(0),
            total_spent: data["total"].as_f64().unwrap_or(0.0),
            last_order_at: None,
            tags: vec![],
            platform_customer_id: customer_data["id"].as_i64().unwrap_or(0).to_string(),
        };

        let line_items = data["orderDetails"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| crate::types::OrderItem {
                        id: item["id"].as_i64().unwrap_or(0).to_string(),
                        product_id: item["productId"].as_i64().unwrap_or(0).to_string(),
                        product_name: item["productName"].as_str().unwrap_or("").to_string(),
                        sku: item["productCode"].as_str().map(String::from),
                        variant_id: item["productId"].as_i64().map(|v| v.to_string()),
                        variant_name: None,
                        quantity: item["quantity"].as_i64().unwrap_or(0) as i32,
                        unit_price: item["price"].as_f64().unwrap_or(0.0),
                        discount: item["discount"]
                            .as_f64()
                            .unwrap_or(0.0),
                        total_price: item["total"]
                            .as_f64()
                            .unwrap_or(0.0),
                        image_url: None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let total = data["total"].as_f64().unwrap_or(0.0);
        let discount = data["discount"].as_f64().unwrap_or(0.0);
        let shipping = data["shippingFee"].as_f64().unwrap_or(0.0);

        Ok(Order {
            id: id.clone(),
            platform: EcommercePlatform::KiotViet,
            external_id: code.to_string(),
            status,
            customer,
            items: line_items,
            pricing: crate::types::OrderPricing {
                subtotal: total + discount - shipping,
                shipping_fee: shipping,
                discount,
                tax: data["tax"].as_f64().unwrap_or(0.0),
                total,
                currency: "VND".to_string(),
            },
            shipping: crate::types::ShippingInfo {
                carrier: data["shipAgentName"].as_str().map(String::from),
                tracking_number: data["trackingNumber"].as_str().map(String::from),
                estimated_delivery: None,
                shipped_at: None,
            },
            fulfillment: crate::types::FulfillmentInfo {
                status: crate::types::FulfillmentStatus::Unfulfilled,
                itemsfulfilled: 0,
                total_items: 0,
            },
            payment: crate::types::PaymentInfo {
                method: data["paymentMethod"].as_str().map(String::from),
                status: if data["isPaid"].as_bool().unwrap_or(false) {
                    crate::types::PaymentStatus::Paid
                } else {
                    crate::types::PaymentStatus::Pending
                },
                transaction_id: None,
                paid_at: None,
            },
            created_at: chrono::DateTime::parse_from_rfc3339(
                data["createdDate"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
            )
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: chrono::Utc::now(),
            notes: data["description"].as_str().map(String::from),
            source: Some("kiotviet".to_string()),
        })
    }

    pub fn parse_product(&self, data: &serde_json::Value) -> Result<Product> {
        let id = data["id"].as_i64().unwrap_or(0).to_string();

        let images = data["images"]
            .as_array()
            .map(|imgs| {
                imgs.iter()
                    .filter_map(|img| img["imageUrl"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let inventory_quantity = data["inventories"]
            .as_array()
            .map(|inv| {
                inv.iter()
                    .map(|i| i["quantity"].as_i64().unwrap_or(0) as i32)
                    .sum()
            })
            .unwrap_or(0);

        Ok(Product {
            id: id.clone(),
            platform: EcommercePlatform::KiotViet,
            external_id: data["code"].as_str().unwrap_or(&id).to_string(),
            name: data["name"].as_str().unwrap_or("").to_string(),
            description: None,
            category: data["category"]["name"].as_str().map(String::from),
            sku: data["code"].as_str().map(String::from),
            variants: vec![],
            images,
            price: crate::types::ProductPrice {
                price: data["basePrice"].as_f64().unwrap_or(0.0),
                compare_at_price: data["orgPrice"].as_f64(),
                cost_price: data["costPrice"].as_f64(),
                currency: "VND".to_string(),
            },
            inventory: crate::types::InventoryInfo {
                tracking: true,
                quantity: inventory_quantity,
                available: inventory_quantity,
                reserved: 0,
                locations: vec![],
            },
            status: if data["isActive"].as_bool().unwrap_or(true) {
                crate::types::ProductStatus::Active
            } else {
                crate::types::ProductStatus::Inactive
            },
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kiotviet_adapter_creation() {
        let adapter = KiotVietAdapter::new("client_id", "client_secret", "retailer");
        assert_eq!(adapter.client_id, "client_id");
        assert_eq!(adapter.retailer, "retailer");
    }

    #[test]
    fn test_get_command() {
        let adapter = KiotVietAdapter::new("client_id", "client_secret", "retailer");
        let (cmd, args, env) = adapter.get_command();

        assert_eq!(cmd, "npx");
        assert!(args.contains(&"kiotviet-mcp".to_string()));
        assert_eq!(env.get("KIOTVIET_CLIENT_ID"), Some(&"client_id".to_string()));
        assert_eq!(env.get("KIOTVIET_RETAILER"), Some(&"retailer".to_string()));
    }

    #[test]
    fn test_presets() {
        let adapter = KiotVietAdapter::new("client_id", "client_secret", "retailer");
        let presets = adapter.get_presets();

        assert!(presets.contains(&"preset.default"));
        assert!(presets.contains(&"preset.readonly"));
        assert!(presets.contains(&"preset.inventory"));
    }
}
