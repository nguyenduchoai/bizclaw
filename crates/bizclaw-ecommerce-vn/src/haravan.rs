//! Haravan MCP Integration
//!
//! Connects to Haravan MCP server with Smart Tools support.

use crate::types::{Order, OrderStatus, Product, Customer, EcommercePlatform};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct HaravanAdapter {
    access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaravanConfig {
    pub access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaravanSmartTools {
    pub enabled: bool,
}

impl HaravanAdapter {
    pub fn new(access_token: &str) -> Self {
        Self {
            access_token: access_token.to_string(),
        }
    }

    pub fn from_config(config: &HaravanConfig) -> Self {
        Self::new(&config.access_token)
    }

    pub fn get_command(&self) -> (&str, Vec<String>, std::collections::HashMap<String, String>) {
        let mut env = std::collections::HashMap::new();
        env.insert("HARAVAN_ACCESS_TOKEN".to_string(), self.access_token.clone());

        (
            "npx",
            vec![
                "-y".to_string(),
                "haravan-mcp".to_string(),
                "mcp".to_string(),
                "-t".to_string(),
                self.access_token.clone(),
                "--tools".to_string(),
                "preset.smart,preset.default".to_string(),
            ],
            env,
        )
    }

    pub fn get_tools(&self) -> Vec<&'static str> {
        vec![
            "hrv_orders_summary",
            "hrv_top_products",
            "hrv_customer_segments",
            "hrv_inventory_health",
            "hrv_stock_reorder_plan",
            "hrv_orders_list",
            "hrv_orders_get",
            "hrv_customers_list",
            "hrv_customers_get",
            "hrv_products_list",
            "hrv_products_get",
            "hrv_inventory_get",
        ]
    }

    pub fn get_smart_tools(&self) -> Vec<&'static str> {
        vec![
            "hrv_orders_summary",
            "hrv_top_products",
            "hrv_customer_segments",
            "hrv_inventory_health",
            "hrv_stock_reorder_plan",
            "hrv_inventory_imbalance",
            "hrv_order_cycle_time",
        ]
    }

    pub fn parse_order(&self, data: &serde_json::Value) -> Result<Order> {
        let id = data["id"].as_i64().unwrap_or(0).to_string();
        let name = data["name"].as_str().unwrap_or(&id);
        let financial_status = data["financial_status"].as_str().unwrap_or("pending");
        let fulfillment_status = data["fulfillment_status"].as_str().unwrap_or("");

        let customer = Customer {
            id: data["customer"]["id"].as_i64().unwrap_or(0).to_string(),
            name: data["customer"]["first_name"]
                .as_str()
                .map(|f| format!("{} {}", f, data["customer"]["last_name"].as_str().unwrap_or("")))
                .unwrap_or_else(|| data["customer"]["name"].as_str().unwrap_or("").to_string()),
            email: data["customer"]["email"].as_str().map(String::from),
            phone: data["customer"]["phone"].as_str().map(String::from),
            address: None,
            total_orders: data["customer"]["orders_count"].as_i64().unwrap_or(0) as i32,
            total_spent: data["customer"]["total_spent"]
                .as_f64()
                .unwrap_or(0.0),
            last_order_at: None,
            tags: data["customer"]["tags"]
                .as_str()
                .map(|t| t.split(',').map(String::from).collect())
                .unwrap_or_default(),
            platform_customer_id: data["customer"]["id"].as_i64().unwrap_or(0).to_string(),
        };

        let line_items: Vec<crate::types::OrderItem> = data["line_items"]
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
                        discount: item["total_discount"]
                            .as_f64()
                            .unwrap_or(0.0),
                        total_price: item["price"]
                            .as_f64()
                            .unwrap_or(0.0)
                            * item["quantity"].as_f64().unwrap_or(1.0),
                        image_url: item["image"].as_str().map(String::from),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let line_items_count = line_items.len() as i32;

        let subtotal = data["subtotal_price"].as_f64().unwrap_or(0.0);
        let total_shipping = data["total_shipping_price_set"]["shop_money"]["amount"]
            .as_f64()
            .unwrap_or(0.0);
        let total_discounts = data["total_discounts"].as_f64().unwrap_or(0.0);
        let total_price = data["total_price"].as_f64().unwrap_or(0.0);
        let total_tax = data["total_tax"].as_f64().unwrap_or(0.0);

        let status = if !fulfillment_status.is_empty() && fulfillment_status != "none" {
            OrderStatus::from_platform(fulfillment_status, "haravan")
        } else {
            OrderStatus::from_platform(financial_status, "haravan")
        };

        Ok(Order {
            id: id.clone(),
            platform: EcommercePlatform::Haravan,
            external_id: name.to_string(),
            status,
            customer,
            items: line_items,
            pricing: crate::types::OrderPricing {
                subtotal,
                shipping_fee: total_shipping,
                discount: total_discounts,
                tax: total_tax,
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
                status: if fulfillment_status == "fulfilled" {
                    crate::types::FulfillmentStatus::Fulfilled
                } else {
                    crate::types::FulfillmentStatus::Unfulfilled
                },
                itemsfulfilled: data["fulfillments"]
                    .as_array()
                    .map(|f| f.len() as i32)
                    .unwrap_or(0),
                total_items: line_items_count,
            },
            payment: crate::types::PaymentInfo {
                method: data["payment_details"]["method"].as_str().map(String::from),
                status: crate::types::PaymentStatus::from_platform(financial_status),
                transaction_id: data["gateway"].as_str().map(String::from),
                paid_at: if financial_status == "paid" {
                    Some(chrono::Utc::now())
                } else {
                    None
                },
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

        let variants: Vec<crate::types::ProductVariant> = data["variants"]
            .as_array()
            .map(|vars| {
                vars.iter()
                    .map(|v| crate::types::ProductVariant {
                        id: v["id"].as_i64().unwrap_or(0).to_string(),
                        external_id: v["id"].as_i64().unwrap_or(0).to_string(),
                        sku: v["sku"].as_str().map(String::from),
                        name: v["title"].as_str().map(String::from),
                        price: v["price"].as_f64().unwrap_or(0.0),
                        compare_at_price: v["compare_at_price"].as_f64(),
                        inventory_quantity: v["inventory_quantity"].as_i64().unwrap_or(0) as i32,
                        image_url: v["image"].as_str().map(String::from),
                        options: vec![],
                    })
                    .collect()
            })
            .unwrap_or_default();

        let images = data["images"]
            .as_array()
            .map(|imgs| {
                imgs.iter()
                    .filter_map(|img| img["src"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Product {
            id: id.clone(),
            platform: EcommercePlatform::Haravan,
            external_id: id,
            name: data["title"].as_str().unwrap_or("").to_string(),
            description: data["body_html"].as_str().map(String::from),
            category: data["product_type"].as_str().map(String::from),
            sku: variants
                .first()
                .and_then(|v| v.sku.clone()),
            variants,
            images,
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
            status: if data["status"].as_str() == Some("active") {
                crate::types::ProductStatus::Active
            } else {
                crate::types::ProductStatus::Inactive
            },
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

    pub fn parse_orders_summary(&self, data: &serde_json::Value) -> Option<crate::analytics::RevenueAnalytics> {
        let total_revenue = data["total_revenue"]
            .as_str()
            .and_then(|s| s.replace(",", "").replace(" VND", "").parse::<f64>().ok())
            .unwrap_or(0.0);

        let aov = data["aov"]
            .as_str()
            .and_then(|s| s.replace(",", "").replace(" VND", "").parse::<f64>().ok())
            .unwrap_or(0.0);

        let orders = data["total_orders"].as_i64().unwrap_or(0) as i32;

        Some(crate::analytics::RevenueAnalytics {
            total_revenue,
            total_orders: orders,
            average_order_value: aov,
            total_shipping_fee: 0.0,
            total_discount: 0.0,
            net_revenue: total_revenue,
            revenue_by_day: vec![],
            revenue_growth: data["comparison_with_last_month"]
                .as_str()
                .and_then(|s| {
                    let num_str = s.trim_start_matches(['+', '-']).trim_end_matches('%');
                    num_str.parse::<f32>().ok().map(|n| {
                        if s.starts_with('-') { -n } else { n }
                    })
                }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haravan_adapter_creation() {
        let adapter = HaravanAdapter::new("test_token");
        assert_eq!(adapter.access_token, "test_token");
    }

    #[test]
    fn test_get_command() {
        let adapter = HaravanAdapter::new("test_token");
        let (cmd, args, env) = adapter.get_command();

        assert_eq!(cmd, "npx");
        assert!(args.contains(&"haravan-mcp".to_string()));
        assert_eq!(
            env.get("HARAVAN_ACCESS_TOKEN"),
            Some(&"test_token".to_string())
        );
    }

    #[test]
    fn test_smart_tools() {
        let adapter = HaravanAdapter::new("test_token");
        let smart_tools = adapter.get_smart_tools();

        assert!(smart_tools.contains(&"hrv_orders_summary"));
        assert!(smart_tools.contains(&"hrv_top_products"));
        assert!(smart_tools.contains(&"hrv_customer_segments"));
    }
}
