//! Unified Customer View across multiple e-commerce platforms

use crate::types::{Customer, EcommercePlatform};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedCustomer {
    pub unified_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub addresses: Vec<AddressMatch>,
    pub platforms: Vec<PlatformCustomer>,
    pub total_orders: i32,
    pub total_spent: f64,
    pub average_order_value: f64,
    pub first_order_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_order_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
    pub risk_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressMatch {
    pub full_address: String,
    pub city: Option<String>,
    pub district: Option<String>,
    pub ward: Option<String>,
    pub source_platform: EcommercePlatform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCustomer {
    pub platform: EcommercePlatform,
    pub platform_customer_id: String,
    pub orders_count: i32,
    pub total_spent: f64,
    pub last_order_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerMatch {
    pub match_type: MatchType,
    pub confidence: f32,
    pub unified_customer: Option<UnifiedCustomer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    ExactPhone,
    ExactEmail,
    ExactName,
    FuzzyPhone,
    FuzzyName,
    NewCustomer,
    Ambiguous,
}

pub struct CustomerUnifier {
    similarity_threshold: f32,
}

impl CustomerUnifier {
    pub fn new(similarity_threshold: f32) -> Self {
        Self {
            similarity_threshold,
        }
    }

    pub fn match_customer(
        &self,
        incoming: &Customer,
        existing: &[UnifiedCustomer],
    ) -> CustomerMatch {
        for customer in existing {
            if let Some(score) = self.check_phone_match(&incoming.phone, customer) {
                if score >= 0.95 {
                    return CustomerMatch {
                        match_type: MatchType::ExactPhone,
                        confidence: score,
                        unified_customer: Some(customer.clone()),
                    };
                } else if score >= self.similarity_threshold {
                    return CustomerMatch {
                        match_type: MatchType::FuzzyPhone,
                        confidence: score,
                        unified_customer: Some(customer.clone()),
                    };
                }
            }

            if let Some(score) = self.check_email_match(&incoming.email, customer) {
                if score >= 0.95 {
                    return CustomerMatch {
                        match_type: MatchType::ExactEmail,
                        confidence: score,
                        unified_customer: Some(customer.clone()),
                    };
                }
            }

            if let Some(score) = self.check_name_match(&incoming.name, customer) {
                if score >= 0.90 {
                    return CustomerMatch {
                        match_type: MatchType::FuzzyName,
                        confidence: score,
                        unified_customer: Some(customer.clone()),
                    };
                }
            }
        }

        CustomerMatch {
            match_type: MatchType::NewCustomer,
            confidence: 1.0,
            unified_customer: None,
        }
    }

    fn check_phone_match(&self, phone: &Option<String>, customer: &UnifiedCustomer) -> Option<f32> {
        let phone = phone.as_ref()?;
        let normalized = normalize_phone(phone);

        for platform_customer in &customer.platforms {
            if let Some(customer_phone) = &customer.phone {
                let customer_normalized = normalize_phone(customer_phone);
                if normalized == customer_normalized {
                    return Some(1.0);
                }

                let similarity = jaro_winkler_similarity(&normalized, &customer_normalized);
                if similarity >= 0.85 {
                    return Some(similarity);
                }
            }
        }

        None
    }

    fn check_email_match(&self, email: &Option<String>, customer: &UnifiedCustomer) -> Option<f32> {
        let email = email.as_ref()?.to_lowercase();
        let customer_email = customer.email.as_ref()?.to_lowercase();

        if email == customer_email {
            Some(1.0)
        } else {
            None
        }
    }

    fn check_name_match(&self, name: &str, customer: &UnifiedCustomer) -> Option<f32> {
        let similarity = jaro_winkler_similarity(name, &customer.name);
        if similarity >= self.similarity_threshold {
            Some(similarity)
        } else {
            None
        }
    }

    pub fn merge_customers(&self, customers: Vec<(EcommercePlatform, Customer)>) -> UnifiedCustomer {
        let mut unified = UnifiedCustomer {
            unified_id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            email: None,
            phone: None,
            addresses: Vec::new(),
            platforms: Vec::new(),
            total_orders: 0,
            total_spent: 0.0,
            average_order_value: 0.0,
            first_order_at: None,
            last_order_at: None,
            tags: Vec::new(),
            risk_score: None,
        };

        for (platform, customer) in customers {
            if unified.name.is_empty() {
                unified.name = customer.name.clone();
            }

            if unified.email.is_none() {
                unified.email = customer.email.clone();
            } else if let Some(email) = &customer.email {
                if unified.email.as_ref() != Some(email) {
                    unified.email = Some(format!("{}; {}", unified.email.unwrap(), email));
                }
            }

            if unified.phone.is_none() {
                unified.phone = customer.phone.clone();
            }

            if let Some(address) = &customer.address {
                unified.addresses.push(AddressMatch {
                    full_address: address.full_address(),
                    city: address.city.clone(),
                    district: address.district.clone(),
                    ward: address.ward.clone(),
                    source_platform: platform,
                });
            }

            unified.platforms.push(PlatformCustomer {
                platform,
                platform_customer_id: customer.platform_customer_id.clone(),
                orders_count: customer.total_orders,
                total_spent: customer.total_spent,
                last_order_at: customer.last_order_at,
            });

            unified.total_orders += customer.total_orders;
            unified.total_spent += customer.total_spent;
            unified.tags.extend(customer.tags);

            if unified.first_order_at.is_none()
                || customer.last_order_at < unified.first_order_at
            {
                unified.first_order_at = customer.last_order_at;
            }

            if unified.last_order_at.is_none()
                || customer.last_order_at > unified.last_order_at
            {
                unified.last_order_at = customer.last_order_at;
            }
        }

        if unified.total_orders > 0 {
            unified.average_order_value = unified.total_spent / unified.total_orders as f64;
        }

        unified.risk_score = Some(self.calculate_risk_score(&unified));

        unified
    }

    fn calculate_risk_score(&self, customer: &UnifiedCustomer) -> f32 {
        let mut score = 0.5f32;

        if customer.total_orders == 0 {
            score += 0.2;
        } else if customer.total_orders >= 10 {
            score -= 0.2;
        }

        if customer.total_spent < 100_000.0 {
            score += 0.1;
        } else if customer.total_spent >= 5_000_000.0 {
            score -= 0.15;
        }

        let days_since_last_order = customer
            .last_order_at
            .map(|dt| (chrono::Utc::now() - dt).num_days() as i32)
            .unwrap_or(365);

        if days_since_last_order > 180 {
            score += 0.15;
        } else if days_since_last_order < 30 {
            score -= 0.1;
        }

        score.max(0.0).min(1.0)
    }
}

fn normalize_phone(phone: &str) -> String {
    phone.chars()
        .filter(|c| c.is_ascii_digit())
        .collect()
}

fn jaro_winkler_similarity(s1: &str, s2: &str) -> f32 {
    if s1.is_empty() && s2.is_empty() {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    let match_distance = (len1.max(len2) / 2) - 1;
    let mut s1_matches = vec![false; len1];
    let mut s2_matches = vec![false; len2];

    let mut matches = 0;
    let mut transpositions = 0;

    for i in 0..len1 {
        let start = if i > match_distance {
            i - match_distance
        } else {
            0
        };
        let end = if i + match_distance < len2 {
            i + match_distance + 1
        } else {
            len2
        };

        for j in start..end {
            if s2_matches[j] || s1_chars[i] != s2_chars[j] {
                continue;
            }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut k = 0;
    for i in 0..len1 {
        if !s1_matches[i] {
            continue;
        }
        while !s2_matches[k] {
            k += 1;
        }
        if s1_chars[i] != s2_chars[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let jaro = (matches as f32 / len1 as f32
        + matches as f32 / len2 as f32
        + (matches as f32 - transpositions as f32 / 2.0) / matches as f32)
        / 3.0;

    let prefix_len = (0..4)
        .take_while(|&i| i < len1.min(len2) && s1_chars[i] == s2_chars[i])
        .count();

    jaro + (prefix_len as f32 * 0.1 * (1.0 - jaro))
}

impl Default for CustomerUnifier {
    fn default() -> Self {
        Self::new(0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_phone_normalization() {
        assert_eq!(normalize_phone("0912-345-678"), "0912345678");
        assert_eq!(normalize_phone("(0912) 345 678"), "0912345678");
        assert_eq!(normalize_phone("+84 912 345 678"), "84912345678");
    }

    #[test]
    fn test_jaro_winkler_exact() {
        assert!((jaro_winkler_similarity("hello", "hello") - 1.0).abs() < 0.001);
        assert!((jaro_winkler_similarity("", "") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_jaro_winkler_similar() {
        let similarity = jaro_winkler_similarity("Nguyễn Văn An", "Nguyễn Văn An");
        assert!(similarity > 0.9);
    }
}
