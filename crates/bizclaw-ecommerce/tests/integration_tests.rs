#[cfg(test)]
mod tests {
    use bizclaw_ecommerce::{
        EcommerceConfig, ShopeeConfig, TiktokConfig,
        types::{
            DailySales, InventoryItem, Order, OrderItem, OrderStatus, Product, ProductStatus,
            SalesReport, TopProduct,
        },
    };

    #[test]
    fn test_ecommerce_config_serialization() {
        let config = EcommerceConfig {
            tiktok: Some(TiktokConfig {
                app_id: "test_app_id".to_string(),
                app_secret: "test_secret".to_string(),
                access_token: None,
                shop_id: None,
            }),
            shopee: Some(ShopeeConfig {
                partner_id: 123456,
                shop_id: 789012,
                api_key: "api_key".to_string(),
                secret_key: "secret_key".to_string(),
            }),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test_app_id"));
        assert!(json.contains("123456"));

        let parsed: EcommerceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tiktok.as_ref().unwrap().app_id, "test_app_id");
        assert_eq!(parsed.shopee.as_ref().unwrap().partner_id, 123456);
    }

    #[test]
    fn test_order_creation() {
        let order = Order {
            id: "ORD001".to_string(),
            platform: "tiktok_shop".to_string(),
            status: OrderStatus::Pending,
            customer_name: "Nguyen Van A".to_string(),
            customer_phone: Some("0909123456".to_string()),
            customer_address: Some("123 ABC Street, District 1, HCMC".to_string()),
            total_amount: 500000.0,
            shipping_fee: 30000.0,
            discount: 50000.0,
            items: vec![OrderItem {
                id: "ITEM001".to_string(),
                product_id: "PROD001".to_string(),
                product_name: "Product A".to_string(),
                sku: Some("SKU001".to_string()),
                quantity: 2,
                unit_price: 250000.0,
                discount: 25000.0,
                total_price: 475000.0,
                image_url: None,
            }],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            notes: None,
        };

        assert_eq!(order.items.len(), 1);
        assert_eq!(order.total_amount, 500000.0);
        assert_eq!(order.items[0].quantity, 2);
    }

    #[test]
    fn test_order_status_mapping() {
        assert_eq!(OrderStatus::from("pending"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from("confirmed"), OrderStatus::Confirmed);
        assert_eq!(OrderStatus::from("shipped"), OrderStatus::Shipped);
        assert_eq!(OrderStatus::from("delivered"), OrderStatus::Delivered);
        assert_eq!(OrderStatus::from("cancelled"), OrderStatus::Cancelled);
        assert_eq!(OrderStatus::from("unknown"), OrderStatus::Unknown);
    }

    #[test]
    fn test_product_creation() {
        let product = Product {
            id: "PROD001".to_string(),
            platform: "shopee".to_string(),
            name: "Test Product".to_string(),
            description: Some("A test product description".to_string()),
            category: Some("Electronics".to_string()),
            sku: Some("SKU-TEST-001".to_string()),
            price: 999000.0,
            original_price: Some(1299000.0),
            stock: 100,
            images: vec!["https://example.com/image1.jpg".to_string()],
            status: ProductStatus::Active,
            rating: Some(4.5),
            sold_count: 250,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert!(product.sku.is_some());
        assert_eq!(product.stock, 100);
        assert!(product.original_price > Some(product.price));
    }

    #[test]
    fn test_inventory_item_status() {
        let low_stock_item = InventoryItem {
            product_id: "PROD001".to_string(),
            sku: Some("SKU001".to_string()),
            warehouse_id: Some("WH001".to_string()),
            quantity: 5,
            reserved_quantity: 0,
            available_quantity: 5,
            updated_at: chrono::Utc::now(),
        };

        let healthy_item = InventoryItem {
            product_id: "PROD002".to_string(),
            sku: Some("SKU002".to_string()),
            warehouse_id: Some("WH001".to_string()),
            quantity: 50,
            reserved_quantity: 10,
            available_quantity: 40,
            updated_at: chrono::Utc::now(),
        };

        assert!(low_stock_item.available_quantity < 10);
        assert!(healthy_item.available_quantity >= 10);
        assert_eq!(
            healthy_item.quantity - healthy_item.reserved_quantity,
            healthy_item.available_quantity
        );
    }

    #[test]
    fn test_sales_report_calculations() {
        let report = SalesReport {
            platform: "shopee".to_string(),
            period_start: chrono::Utc::now(),
            period_end: chrono::Utc::now(),
            total_orders: 500,
            total_revenue: 100_000_000.0,
            total_shipping_fee: 15_000_000.0,
            total_discount: 10_000_000.0,
            total_refunds: 5_000_000.0,
            net_revenue: 70_000_000.0,
            top_products: vec![
                TopProduct {
                    product_id: "P001".to_string(),
                    product_name: "Product A".to_string(),
                    quantity_sold: 200,
                    revenue: 40_000_000.0,
                },
                TopProduct {
                    product_id: "P002".to_string(),
                    product_name: "Product B".to_string(),
                    quantity_sold: 150,
                    revenue: 30_000_000.0,
                },
            ],
            daily_breakdown: vec![DailySales {
                date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                orders: 50,
                revenue: 10_000_000.0,
            }],
        };

        assert_eq!(report.total_orders, 500);
        assert_eq!(report.top_products.len(), 2);
        assert_eq!(report.daily_breakdown.len(), 1);
    }

    #[test]
    fn test_product_status() {
        assert_eq!(ProductStatus::Active, ProductStatus::Active);
        assert_eq!(ProductStatus::Inactive, ProductStatus::Inactive);
        assert_eq!(ProductStatus::Deleted, ProductStatus::Deleted);
    }

    #[test]
    fn test_order_serialization_roundtrip() {
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
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            notes: None,
        };

        let json = serde_json::to_string(&order).unwrap();
        let parsed: Order = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "ORD123");
        assert_eq!(parsed.status, OrderStatus::Pending);
    }
}
