//! Meta Ads Ad Set Management

use crate::types::MetaAdsSettings;
use crate::types::{AdsetStatus, GeoLocations, OptimizationGoal, BillingEvent, Targeting};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AdsetManager {
    settings: MetaAdsSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAdsetRequest {
    pub campaign_id: String,
    pub name: String,
    pub status: String,
    pub daily_budget: Option<f64>,
    pub lifetime_budget: Option<f64>,
    pub targeting: TargetingInput,
    pub optimization_goal: String,
    pub billing_event: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TargetingInput {
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub genders: Option<Vec<i32>>,
    pub geo_locations: GeoLocationsInput,
    pub interests: Option<Vec<InterestInput>>,
    pub placements: Option<PlacementsInput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeoLocationsInput {
    pub countries: Option<Vec<String>>,
    pub cities: Option<Vec<serde_json::Value>>,
    pub regions: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InterestInput {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlacementsInput {
    pub platform_positions: Option<Vec<serde_json::Value>>,
    pub publisher_platforms: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAdsetRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub daily_budget: Option<f64>,
    pub lifetime_budget: Option<f64>,
    pub targeting: Option<TargetingInput>,
}

impl AdsetManager {
    pub fn new(settings: MetaAdsSettings) -> Self {
        Self { settings }
    }

    pub async fn create_adset(&self, request: CreateAdsetRequest) -> Result<crate::types::AdSet> {
        tracing::info!("Creating adset: {} for campaign {}", request.name, request.campaign_id);

        let geo = GeoLocations {
            countries: request.targeting.geo_locations.countries.clone(),
            cities: None,
            regions: None,
            zips: None,
        };

        let targeting = Targeting {
            age_min: request.targeting.age_min,
            age_max: request.targeting.age_max,
            genders: request.targeting.genders.clone(),
            geo_locations: geo,
            interests: request.targeting.interests.as_ref().map(|interests| {
                interests.iter().map(|i| crate::types::Interest {
                    id: i.id.clone(),
                    name: i.name.clone(),
                }).collect()
            }),
            behaviors: None,
            exclusions: None,
        };

        Ok(crate::types::AdSet {
            id: uuid::Uuid::new_v4().to_string(),
            campaign_id: request.campaign_id,
            name: request.name,
            status: AdsetStatus::Paused,
            daily_budget: request.daily_budget,
            lifetime_budget: request.lifetime_budget,
            targeting,
            optimization_goal: OptimizationGoal::CONVERSIONS,
            billing_event: BillingEvent::IMPRESSIONS,
            start_time: request.start_time.as_ref().and_then(|t| {
                chrono::DateTime::parse_from_rfc3339(t).ok()
            }).map(|dt| dt.with_timezone(&Utc)),
            end_time: request.end_time.as_ref().and_then(|t| {
                chrono::DateTime::parse_from_rfc3339(t).ok()
            }).map(|dt| dt.with_timezone(&Utc)),
        })
    }

    pub async fn get_adset(&self, adset_id: &str) -> Result<Option<crate::types::AdSet>> {
        tracing::debug!("Fetching adset: {}", adset_id);
        Ok(None)
    }

    pub async fn list_adsets(&self, campaign_id: &str) -> Result<Vec<crate::types::AdSet>> {
        tracing::debug!("Listing adsets for campaign: {}", campaign_id);
        Ok(vec![])
    }

    pub async fn update_adset(&self, adset_id: &str, request: UpdateAdsetRequest) -> Result<crate::types::AdSet> {
        tracing::info!("Updating adset: {}", adset_id);
        let mut adset = self.get_adset(adset_id).await?.unwrap();

        if let Some(name) = request.name {
            adset.name = name;
        }

        if let Some(status) = request.status {
            adset.status = match status.to_lowercase().as_str() {
                "active" => AdsetStatus::Active,
                "paused" => AdsetStatus::Paused,
                "archived" => AdsetStatus::Archived,
                _ => AdsetStatus::Paused,
            };
        }

        if let Some(daily_budget) = request.daily_budget {
            adset.daily_budget = Some(daily_budget);
        }

        if let Some(lifetime_budget) = request.lifetime_budget {
            adset.lifetime_budget = Some(lifetime_budget);
        }

        Ok(adset)
    }

    pub async fn pause_adset(&self, adset_id: &str) -> Result<crate::types::AdSet> {
        tracing::info!("Pausing adset: {}", adset_id);
        let request = UpdateAdsetRequest {
            name: None,
            status: Some("PAUSED".to_string()),
            daily_budget: None,
            lifetime_budget: None,
            targeting: None,
        };
        self.update_adset(adset_id, request).await
    }

    pub async fn resume_adset(&self, adset_id: &str) -> Result<crate::types::AdSet> {
        tracing::info!("Resuming adset: {}", adset_id);
        let request = UpdateAdsetRequest {
            name: None,
            status: Some("ACTIVE".to_string()),
            daily_budget: None,
            lifetime_budget: None,
            targeting: None,
        };
        self.update_adset(adset_id, request).await
    }
}
