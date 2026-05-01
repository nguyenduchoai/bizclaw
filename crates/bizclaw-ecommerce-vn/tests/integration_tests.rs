use bizclaw_ecommerce_vn::{
    unified::{AddressMatch, PlatformCustomer},
    Address, Customer, CustomerUnifier, EcommercePlatform,
    FulfillmentStatus, Order, OrderItem, OrderPricing, OrderStatus,
    UnifiedCustomer,
};
use chrono::Utc;

#[cfg(test)]
mod platform_tests {
    use super::*;

    #[test]
    fn test_platform_code() {
        assert_eq!(EcommercePlatform::Sapo.code(), "sapo");
        assert_eq!(EcommercePlatform::Haravan.code(), "haravan");
        assert_eq!(EcommercePlatform::KiotViet.code(), "kiotviet");
        assert_eq!(EcommercePlatform::LadiSales.code(), "ladisales");
    }

    #[test]
    fn test_platform_from_code() {
        assert_eq!(EcommercePlatform::from_code("sapo"), EcommercePlatform::Sapo);
        assert_eq!(EcommercePlatform::from_code("haravan"), EcommercePlatform::Haravan);
        assert_eq!(EcommercePlatform::from_code("kiotviet"), EcommercePlatform::KiotViet);
        assert_eq!(EcommercePlatform::from_code("ladisales"), EcommercePlatform::LadiSales);
        assert_eq!(EcommercePlatform::from_code("unknown"), EcommercePlatform::Unknown);
    }

    #[test]
    fn test_platform_display_name() {
        assert_eq!(EcommercePlatform::Sapo.display_name(), "Sapo POS & Online");
        assert_eq!(EcommercePlatform::Haravan.display_name(), "Haravan");
        assert_eq!(EcommercePlatform::KiotViet.display_name(), "KiotViet");
        assert_eq!(EcommercePlatform::LadiSales.display_name(), "LadiSales");
    }

    #[test]
    fn test_platform_api_docs_url() {
        assert!(EcommercePlatform::Sapo.api_docs_url().contains("sapo"));
        assert!(EcommercePlatform::Haravan.api_docs_url().contains("haravan"));
        assert!(EcommercePlatform::KiotViet.api_docs_url().contains("kiotviet"));
        assert!(EcommercePlatform::LadiSales.api_docs_url().contains("ldpform"));
    }
}

#[cfg(test)]
mod order_status_tests {
    use super::*;

    #[test]
    fn test_order_status_from_sapo() {
        assert_eq!(OrderStatus::from_platform("pending", "sapo"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from_platform("confirmed", "sapo"), OrderStatus::Confirmed);
        assert_eq!(OrderStatus::from_platform("shipped", "sapo"), OrderStatus::Shipped);
        assert_eq!(OrderStatus::from_platform("delivered", "sapo"), OrderStatus::Delivered);
        assert_eq!(OrderStatus::from_platform("cancelled", "sapo"), OrderStatus::Cancelled);
    }

    #[test]
    fn test_order_status_from_haravan() {
        assert_eq!(OrderStatus::from_platform("open", "haravan"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from_platform("closed", "haravan"), OrderStatus::Delivered);
    }

    #[test]
    fn test_order_status_from_kiotviet() {
        assert_eq!(OrderStatus::from_platform("Pending", "kiotviet"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from_platform("Completed", "kiotviet"), OrderStatus::Delivered);
    }

    #[test]
    fn test_order_status_to_platform() {
        assert_eq!(OrderStatus::Pending.to_sapo(), "pending");
        assert_eq!(OrderStatus::Delivered.to_sapo(), "delivered");
        assert_eq!(OrderStatus::Pending.to_haravan(), "open");
        assert_eq!(OrderStatus::Delivered.to_haravan(), "closed");
        assert_eq!(OrderStatus::Pending.to_kiotviet(), "Pending");
        assert_eq!(OrderStatus::Delivered.to_kiotviet(), "Completed");
    }
}

#[cfg(test)]
mod address_tests {
    use super::*;

    #[test]
    fn test_address_full_address() {
        let address = Address {
            street: Some("123 Đường Test".to_string()),
            city: Some("TP HCM".to_string()),
            district: Some("Quận 1".to_string()),
            ward: Some("Phường 1".to_string()),
            postal_code: None,
            country: Some("Vietnam".to_string()),
        };

        assert_eq!(address.full_address(), "123 Đường Test, Phường 1, Quận 1, TP HCM");
    }

    #[test]
    fn test_address_partial() {
        let address = Address {
            street: Some("456 Test".to_string()),
            city: Some("Hà Nội".to_string()),
            district: None,
            ward: None,
            postal_code: None,
            country: None,
        };

        assert_eq!(address.full_address(), "456 Test, Hà Nội");
    }
}

#[cfg(test)]
mod customer_tests {
    use super::*;

    #[test]
    fn test_unified_customer_creation() {
        let customer = UnifiedCustomer {
            unified_id: "cust_001".to_string(),
            name: "Test Customer".to_string(),
            email: Some("test@example.com".to_string()),
            phone: Some("0912345678".to_string()),
            addresses: vec![AddressMatch {
                full_address: "123 Test, Q1, HCM".to_string(),
                city: Some("HCM".to_string()),
                district: Some("Q1".to_string()),
                ward: None,
                source_platform: EcommercePlatform::Sapo,
            }],
            platforms: vec![
                PlatformCustomer {
                    platform: EcommercePlatform::Sapo,
                    platform_customer_id: "PCUST_SAPO".to_string(),
                    orders_count: 5,
                    total_spent: 500000.0,
                    last_order_at: Some(Utc::now()),
                },
                PlatformCustomer {
                    platform: EcommercePlatform::Haravan,
                    platform_customer_id: "PCUST_HARAVAN".to_string(),
                    orders_count: 3,
                    total_spent: 300000.0,
                    last_order_at: Some(Utc::now()),
                },
            ],
            total_orders: 8,
            total_spent: 800000.0,
            average_order_value: 100000.0,
            first_order_at: Some(Utc::now()),
            last_order_at: Some(Utc::now()),
            tags: vec!["vip".to_string()],
            risk_score: Some(0.1),
        };

        assert_eq!(customer.unified_id, "cust_001");
        assert_eq!(customer.platforms.len(), 2);
        assert_eq!(customer.total_orders, 8);
        assert_eq!(customer.total_spent, 800000.0);
    }

    #[test]
    fn test_customer_unifier_creation() {
        let unifier = CustomerUnifier::new(0.85);
        assert!(true);
    }
}

#[cfg(test)]
mod order_tests {
    use super::*;

    #[test]
    fn test_order_item_creation() {
        let item = OrderItem {
            id: "item_001".to_string(),
            product_id: "prod_001".to_string(),
            product_name: "Test Product".to_string(),
            sku: Some("SKU-001".to_string()),
            variant_id: None,
            variant_name: None,
            quantity: 2,
            unit_price: 50000.0,
            discount: 0.0,
            total_price: 100000.0,
            image_url: None,
        };

        assert_eq!(item.product_name, "Test Product");
        assert_eq!(item.quantity, 2);
        assert_eq!(item.total_price, 100000.0);
    }

    #[test]
    fn test_fulfillment_status() {
        assert_eq!(FulfillmentStatus::Unfulfilled, FulfillmentStatus::Unfulfilled);
        assert_eq!(FulfillmentStatus::PartiallyFulfilled, FulfillmentStatus::PartiallyFulfilled);
        assert_eq!(FulfillmentStatus::Fulfilled, FulfillmentStatus::Fulfilled);
    }
}
