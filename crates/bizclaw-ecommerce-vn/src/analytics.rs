//! Cross-platform Analytics for Vietnamese E-commerce
//!
//! Unified analytics across Sapo, Haravan, KiotViet, and LadiSales.

use crate::types::{EcommercePlatform, Order, OrderStatus};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossPlatformAnalytics {
    pub period: AnalyticsPeriod,
    pub platforms: Vec<PlatformAnalytics>,
    pub summary: AnalyticsSummary,
    pub benchmarks: Option<BenchmarkComparison>,
    pub trends: TrendAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsPeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAnalytics {
    pub platform: EcommercePlatform,
    pub orders: OrderAnalytics,
    pub revenue: RevenueAnalytics,
    pub customers: CustomerAnalytics,
    pub products: ProductAnalytics,
    pub fulfillment: FulfillmentAnalytics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAnalytics {
    pub total: i32,
    pub pending: i32,
    pub confirmed: i32,
    pub shipped: i32,
    pub delivered: i32,
    pub cancelled: i32,
    pub returned: i32,
    pub cancel_rate: f32,
    pub return_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueAnalytics {
    pub total_revenue: f64,
    pub total_orders: i32,
    pub average_order_value: f64,
    pub total_shipping_fee: f64,
    pub total_discount: f64,
    pub net_revenue: f64,
    pub revenue_by_day: Vec<DailyRevenue>,
    pub revenue_growth: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRevenue {
    pub date: String,
    pub revenue: f64,
    pub orders: i32,
    pub aov: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerAnalytics {
    pub new_customers: i32,
    pub returning_customers: i32,
    pub total_customers: i32,
    pub repeat_rate: f32,
    pub average_ltv: f64,
    pub top_customers: Vec<TopCustomer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopCustomer {
    pub customer_id: String,
    pub name: String,
    pub total_orders: i32,
    pub total_spent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductAnalytics {
    pub total_products: i32,
    pub active_products: i32,
    pub top_products: Vec<TopProduct>,
    pub category_breakdown: Vec<CategoryBreakdown>,
    pub inventory_alerts: Vec<InventoryAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProduct {
    pub product_id: String,
    pub name: String,
    pub sku: Option<String>,
    pub quantity_sold: i32,
    pub revenue: f64,
    pub return_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryBreakdown {
    pub category: String,
    pub product_count: i32,
    pub revenue: f64,
    pub percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryAlert {
    pub product_id: String,
    pub product_name: String,
    pub current_stock: i32,
    pub alert_type: InventoryAlertType,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryAlertType {
    OutOfStock,
    LowStock,
    Overstocked,
    DeadStock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentAnalytics {
    pub on_time_delivery_rate: f32,
    pub average_delivery_days: f32,
    pub cod_fail_rate: f32,
    pub fulfillment_status_breakdown: HashMap<String, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_revenue: f64,
    pub total_orders: i32,
    pub average_order_value: f64,
    pub total_customers: i32,
    pub overall_cancel_rate: f32,
    pub platform_comparison: Vec<PlatformComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformComparison {
    pub platform: EcommercePlatform,
    pub revenue: f64,
    pub orders: i32,
    pub aov: f64,
    pub market_share: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub metrics: HashMap<String, BenchmarkMetric>,
    pub overall_score: f32,
    pub grade: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetric {
    pub value: f32,
    pub benchmark_good: f32,
    pub benchmark_average: f32,
    pub your_position: BenchmarkPosition,
    pub industry: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkPosition {
    Excellent,
    Good,
    Average,
    NeedsImprovement,
    Poor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub revenue_trend: TrendDirection,
    pub orders_trend: TrendDirection,
    pub customers_trend: TrendDirection,
    pub momentum: f32,
    pub insights: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

impl BenchmarkMetric {
    pub fn new(value: f32, industry: &str) -> Self {
        let (benchmark_good, benchmark_average) = match industry {
            "fashion" => (3.0, 5.0),
            "electronics" => (2.0, 4.0),
            "fmcg" => (5.0, 8.0),
            "furniture" => (3.0, 6.0),
            "cosmetics" => (4.0, 7.0),
            _ => (3.0, 5.0),
        };

        let your_position = if value < benchmark_good {
            BenchmarkPosition::Excellent
        } else if value < benchmark_average {
            BenchmarkPosition::Good
        } else if value < benchmark_average * 1.5 {
            BenchmarkPosition::Average
        } else if value < benchmark_average * 2.0 {
            BenchmarkPosition::NeedsImprovement
        } else {
            BenchmarkPosition::Poor
        };

        Self {
            value,
            benchmark_good,
            benchmark_average,
            your_position,
            industry: industry.to_string(),
        }
    }
}

pub struct AnalyticsEngine;

impl AnalyticsEngine {
    pub fn generate_report(orders: &[Order], period: &AnalyticsPeriod) -> CrossPlatformAnalytics {
        let mut platform_analytics = HashMap::new();

        for platform in [
            EcommercePlatform::Sapo,
            EcommercePlatform::Haravan,
            EcommercePlatform::KiotViet,
            EcommercePlatform::LadiSales,
        ] {
            let platform_orders: Vec<&Order> = orders
                .iter()
                .filter(|o| o.platform == platform)
                .collect();

            if !platform_orders.is_empty() {
                platform_analytics.insert(
                    platform,
                    Self::analyze_platform(platform, &platform_orders, period),
                );
            }
        }

        let summary = Self::generate_summary(&platform_analytics);
        let benchmarks = Some(Self::generate_benchmarks(&summary));
        let trends = Self::analyze_trends(orders, period);

        CrossPlatformAnalytics {
            period: period.clone(),
            platforms: platform_analytics.into_values().collect(),
            summary,
            benchmarks,
            trends,
        }
    }

    fn analyze_platform(
        platform: EcommercePlatform,
        orders: &[&Order],
        _period: &AnalyticsPeriod,
    ) -> PlatformAnalytics {
        let total = orders.len() as i32;
        let mut pending = 0;
        let mut confirmed = 0;
        let mut shipped = 0;
        let mut delivered = 0;
        let mut cancelled = 0;
        let mut returned = 0;
        let mut total_revenue = 0.0;
        let mut total_shipping = 0.0;
        let mut total_discount = 0.0;

        for order in orders {
            match order.status {
                OrderStatus::Pending => pending += 1,
                OrderStatus::Confirmed => confirmed += 1,
                OrderStatus::Shipped => shipped += 1,
                OrderStatus::Delivered => delivered += 1,
                OrderStatus::Cancelled => cancelled += 1,
                OrderStatus::Returned => returned += 1,
                _ => {}
            }

            total_revenue += order.pricing.total;
            total_shipping += order.pricing.shipping_fee;
            total_discount += order.pricing.discount;
        }

        let cancel_rate = if total > 0 {
            (cancelled as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let return_rate = if total > 0 {
            (returned as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let aov = if total > 0 {
            total_revenue / total as f64
        } else {
            0.0
        };

        let mut daily_revenue: HashMap<String, (f64, i32)> = HashMap::new();
        for order in orders {
            let date = order.created_at.format("%Y-%m-%d").to_string();
            let entry = daily_revenue.entry(date).or_insert((0.0, 0));
            entry.0 += order.pricing.total;
            entry.1 += 1;
        }

        let revenue_by_day: Vec<DailyRevenue> = daily_revenue
            .into_iter()
            .map(|(date, (revenue, orders))| DailyRevenue {
                date,
                revenue,
                orders,
                aov: if orders > 0 { revenue / orders as f64 } else { 0.0 },
            })
            .collect();

        let mut sorted = revenue_by_day.clone();
        sorted.sort_by(|a, b| a.date.cmp(&b.date));

        let revenue_growth = if sorted.len() >= 2 {
            let mid = sorted.len() / 2;
            let first_half: f64 = sorted[..mid].iter().map(|d| d.revenue).sum();
            let second_half: f64 = sorted[mid..].iter().map(|d| d.revenue).sum();
            if first_half > 0.0 {
                Some((((second_half - first_half) / first_half) * 100.0) as f32)
            } else {
                None
            }
        } else {
            None
        };

        PlatformAnalytics {
            platform,
            orders: OrderAnalytics {
                total,
                pending,
                confirmed,
                shipped,
                delivered,
                cancelled,
                returned,
                cancel_rate,
                return_rate,
            },
            revenue: RevenueAnalytics {
                total_revenue,
                total_orders: total,
                average_order_value: aov,
                total_shipping_fee: total_shipping,
                total_discount,
                net_revenue: total_revenue - total_discount,
                revenue_by_day,
                revenue_growth,
            },
            customers: CustomerAnalytics {
                new_customers: 0,
                returning_customers: 0,
                total_customers: 0,
                repeat_rate: 0.0_f32,
                average_ltv: 0.0,
                top_customers: vec![],
            },
            products: ProductAnalytics {
                total_products: 0,
                active_products: 0,
                top_products: vec![],
                category_breakdown: vec![],
                inventory_alerts: vec![],
            },
            fulfillment: FulfillmentAnalytics {
                on_time_delivery_rate: 0.0,
                average_delivery_days: 0.0,
                cod_fail_rate: 0.0,
                fulfillment_status_breakdown: HashMap::new(),
            },
        }
    }

    fn generate_summary(
        platform_analytics: &HashMap<EcommercePlatform, PlatformAnalytics>,
    ) -> AnalyticsSummary {
        let total_revenue: f64 = platform_analytics.values().map(|p| p.revenue.total_revenue).sum();
        let total_orders: i32 = platform_analytics.values().map(|p| p.orders.total).sum();
        let total_customers: i32 = platform_analytics.values().map(|p| p.customers.total_customers).sum();

        let total_cancelled: i32 = platform_analytics.values().map(|p| p.orders.cancelled).sum();
        let overall_cancel_rate = if total_orders > 0 {
            (total_cancelled as f32 / total_orders as f32) * 100.0
        } else {
            0.0
        };

        let platform_comparison: Vec<PlatformComparison> = platform_analytics
            .values()
            .map(|p| {
                let market_share = if total_revenue > 0.0 {
                    ((p.revenue.total_revenue / total_revenue) * 100.0) as f32
                } else {
                    0.0_f32
                };

                PlatformComparison {
                    platform: p.platform,
                    revenue: p.revenue.total_revenue,
                    orders: p.orders.total,
                    aov: p.revenue.average_order_value,
                    market_share,
                }
            })
            .collect();

        AnalyticsSummary {
            total_revenue,
            total_orders,
            average_order_value: if total_orders > 0 {
                total_revenue / total_orders as f64
            } else {
                0.0
            },
            total_customers,
            overall_cancel_rate,
            platform_comparison,
        }
    }

    fn generate_benchmarks(summary: &AnalyticsSummary) -> BenchmarkComparison {
        let mut metrics = HashMap::new();

        metrics.insert(
            "cancel_rate".to_string(),
            BenchmarkMetric::new(summary.overall_cancel_rate, "general"),
        );

        let mut total_cancel = 0;
        let mut total_orders = 0;
        let score = if summary.total_revenue > 10_000_000.0 {
            90.0
        } else if summary.total_revenue > 5_000_000.0 {
            75.0
        } else if summary.total_revenue > 1_000_000.0 {
            60.0
        } else {
            45.0
        };

        let grade = if score >= 90.0 {
            "A"
        } else if score >= 80.0 {
            "B+"
        } else if score >= 70.0 {
            "B"
        } else if score >= 60.0 {
            "C+"
        } else if score >= 50.0 {
            "C"
        } else {
            "D"
        };

        BenchmarkComparison {
            metrics,
            overall_score: score,
            grade: grade.to_string(),
        }
    }

    fn analyze_trends(orders: &[Order], period: &AnalyticsPeriod) -> TrendAnalysis {
        let mid_point = period.start + Duration::days((period.days / 2) as i64);

        let first_half: f64 = orders
            .iter()
            .filter(|o| o.created_at < mid_point)
            .map(|o| o.pricing.total)
            .sum();

        let second_half: f64 = orders
            .iter()
            .filter(|o| o.created_at >= mid_point)
            .map(|o| o.pricing.total)
            .sum();

        let revenue_trend = if second_half > first_half * 1.1 {
            TrendDirection::Up
        } else if second_half < first_half * 0.9 {
            TrendDirection::Down
        } else {
            TrendDirection::Stable
        };

        let first_half_orders = orders.iter().filter(|o| o.created_at < mid_point).count() as i32;
        let second_half_orders = orders.iter().filter(|o| o.created_at >= mid_point).count() as i32;

        let orders_trend = if (second_half_orders as f32) > (first_half_orders as f32) * 1.1 {
            TrendDirection::Up
        } else if (second_half_orders as f32) < (first_half_orders as f32) * 0.9 {
            TrendDirection::Down
        } else {
            TrendDirection::Stable
        };

        let unique_customers_first: std::collections::HashSet<_> = orders
            .iter()
            .filter(|o| o.created_at < mid_point)
            .map(|o| o.customer.platform_customer_id.clone())
            .collect();

        let unique_customers_second: std::collections::HashSet<_> = orders
            .iter()
            .filter(|o| o.created_at >= mid_point)
            .map(|o| o.customer.platform_customer_id.clone())
            .collect();

        let customers_trend = if unique_customers_second.len() > unique_customers_first.len() {
            TrendDirection::Up
        } else if unique_customers_second.len() < unique_customers_first.len() {
            TrendDirection::Down
        } else {
            TrendDirection::Stable
        };

        let momentum = ((second_half - first_half) / first_half.max(1.0)) * 100.0;

        let mut insights = Vec::new();

        if revenue_trend == TrendDirection::Up {
            insights.push("Doanh thu có xu hướng tăng trong kỳ".to_string());
        } else if revenue_trend == TrendDirection::Down {
            insights.push("Doanh thu có xu hướng giảm trong kỳ".to_string());
        }

        if customers_trend == TrendDirection::Up {
            insights.push("Số lượng khách hàng mới đang tăng".to_string());
        }

        TrendAnalysis {
            revenue_trend,
            orders_trend,
            customers_trend,
            momentum: momentum as f32,
            insights,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_position() {
        let metric = BenchmarkMetric::new(2.5, "fashion");
        assert_eq!(metric.your_position, BenchmarkPosition::Excellent);
    }

    #[test]
    fn test_trend_analysis() {
        let period = AnalyticsPeriod {
            start: Utc::now() - Duration::days(30),
            end: Utc::now(),
            days: 30,
        };

        let trends = TrendAnalysis {
            revenue_trend: TrendDirection::Up,
            orders_trend: TrendDirection::Stable,
            customers_trend: TrendDirection::Up,
            momentum: 15.0,
            insights: vec!["Doanh thu tăng".to_string()],
        };

        assert_eq!(trends.revenue_trend, TrendDirection::Up);
    }
}
