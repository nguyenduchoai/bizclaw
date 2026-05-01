//! Meta Ads Audience Management

use crate::types::MetaAdsSettings;
use crate::types::Audience;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AudienceManager {
    settings: MetaAdsSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomAudienceInput {
    pub name: String,
    pub description: Option<String>,
    pub subtype: String,
    pub customer_file_source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LookalikeAudienceInput {
    pub name: String,
    pub origin_audience_id: String,
    pub lookalike_spec: LookalikeSpec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LookalikeSpec {
    pub type_field: String,
    pub ratio: Option<f64>,
    pub country: String,
    pub is_simplified: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedAudience {
    pub id: String,
    pub name: String,
    pub audience_type: String,
    pub size: i64,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

impl AudienceManager {
    pub fn new(settings: MetaAdsSettings) -> Self {
        Self { settings }
    }

    pub async fn create_custom_audience(
        &self,
        request: CustomAudienceInput,
    ) -> Result<crate::types::CustomAudience> {
        tracing::info!("Creating custom audience: {}", request.name);

        Ok(crate::types::CustomAudience {
            id: uuid::Uuid::new_v4().to_string(),
            name: request.name,
            description: request.description.unwrap_or_default(),
            subtype: request.subtype,
            customer_file_source: request.customer_file_source,
            estimated_count: None,
        })
    }

    pub async fn get_custom_audience(&self, audience_id: &str) -> Result<Option<crate::types::CustomAudience>> {
        tracing::debug!("Fetching custom audience: {}", audience_id);
        Ok(None)
    }

    pub async fn list_custom_audiences(&self) -> Result<Vec<crate::types::CustomAudience>> {
        tracing::debug!("Listing custom audiences");
        Ok(vec![])
    }

    pub async fn delete_custom_audience(&self, audience_id: &str) -> Result<()> {
        tracing::info!("Deleting custom audience: {}", audience_id);
        Ok(())
    }

    pub async fn add_users_to_audience(
        &self,
        audience_id: &str,
        users: Vec<UserData>,
    ) -> Result<()> {
        tracing::info!("Adding {} users to audience: {}", users.len(), audience_id);
        Ok(())
    }

    pub async fn remove_users_from_audience(
        &self,
        audience_id: &str,
        users: Vec<UserData>,
    ) -> Result<()> {
        tracing::info!("Removing {} users from audience: {}", users.len(), audience_id);
        Ok(())
    }

    pub async fn create_lookalike_audience(
        &self,
        request: LookalikeAudienceInput,
    ) -> Result<Audience> {
        tracing::info!(
            "Creating lookalike audience from {} with ratio {:?}",
            request.origin_audience_id,
            request.lookalike_spec.ratio
        );

        Ok(Audience {
            id: uuid::Uuid::new_v4().to_string(),
            name: request.name,
            description: Some(format!(
                "Lookalike from {} with {}% similarity",
                request.origin_audience_id,
                (request.lookalike_spec.ratio.unwrap_or(0.05) * 100.0) as i32
            )),
            audience_size: 0,
            audience_source: Some("lookalike".to_string()),
            lookalike_percentage: request.lookalike_spec.ratio.map(|r| r * 100.0),
            created_at: Some(Utc::now()),
        })
    }

    pub async fn list_saved_audiences(&self) -> Result<Vec<SavedAudience>> {
        tracing::debug!("Listing saved audiences");
        Ok(vec![])
    }

    pub async fn get_audience_size(&self, audience_id: &str) -> Result<i64> {
        tracing::debug!("Getting audience size for: {}", audience_id);
        Ok(0)
    }

    pub async fn estimate_lookalike_size(
        &self,
        origin_audience_id: &str,
        ratio: f64,
        country: &str,
    ) -> Result<i64> {
        tracing::info!(
            "Estimating lookalike size for {} with ratio {} in {}",
            origin_audience_id,
            ratio,
            country
        );
        Ok(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
}

impl UserData {
    pub fn from_email(email: &str) -> Self {
        Self {
            email: Some(email.to_string()),
            phone: None,
            first_name: None,
            last_name: None,
            city: None,
            country: None,
        }
    }

    pub fn from_phone(phone: &str) -> Self {
        Self {
            email: None,
            phone: Some(phone.to_string()),
            first_name: None,
            last_name: None,
            city: None,
            country: None,
        }
    }

    pub fn normalize_phone(&self) -> Option<String> {
        self.phone.as_ref().map(|phone| {
            let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.starts_with("84") {
                format!("+{}", digits)
            } else if digits.starts_with('0') {
                format!("+84{}", &digits[1..])
            } else {
                digits
            }
        })
    }
}
