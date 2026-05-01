//! Meta Ads Creative Management

use crate::types::MetaAdsSettings;
use crate::types::{AdCreative, AdStatus, CallToAction, CreativeType};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AdManager {
    settings: MetaAdsSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAdRequest {
    pub adset_id: String,
    pub name: String,
    pub status: String,
    pub creative: AdCreativeInput,
    pub tracking_specs: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdCreativeInput {
    pub name: String,
    pub creative_type: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub image_url: Option<String>,
    pub video_url: Option<String>,
    pub call_to_action_type: Option<String>,
    pub page_id: Option<String>,
    pub link: Option<String>,
    pub product_set_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAdRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub creative: Option<AdCreativeInput>,
}

impl AdManager {
    pub fn new(settings: MetaAdsSettings) -> Self {
        Self { settings }
    }

    pub async fn create_ad(&self, request: CreateAdRequest) -> Result<crate::types::Ad> {
        tracing::info!("Creating ad: {} for adset {}", request.name, request.adset_id);

        let creative_type = match request.creative.creative_type.to_lowercase().as_str() {
            "image" => CreativeType::Image,
            "video" => CreativeType::Video,
            "carousel" => CreativeType::Carousel,
            "collection" => CreativeType::Collection,
            _ => CreativeType::SingleLink,
        };

        let call_to_action = request.creative.call_to_action_type.as_ref().map(|cta| {
            CallToAction {
                action_type: cta.clone(),
                value: serde_json::json!({
                    "link": request.creative.link.clone().unwrap_or_default()
                }),
            }
        });

        let creative = AdCreative {
            id: uuid::Uuid::new_v4().to_string(),
            name: request.creative.name.clone(),
            creative_type,
            title: request.creative.title.clone(),
            body: request.creative.body.clone(),
            image_url: request.creative.image_url.clone(),
            video_url: request.creative.video_url.clone(),
            call_to_action,
            page_id: request.creative.page_id.clone(),
            product_set_id: request.creative.product_set_id.clone(),
        };

        Ok(crate::types::Ad {
            id: uuid::Uuid::new_v4().to_string(),
            adset_id: request.adset_id,
            name: request.name,
            status: AdStatus::Paused,
            creative,
            tracking_specs: request.tracking_specs,
        })
    }

    pub async fn get_ad(&self, ad_id: &str) -> Result<Option<crate::types::Ad>> {
        tracing::debug!("Fetching ad: {}", ad_id);
        Ok(None)
    }

    pub async fn list_ads(&self, adset_id: &str) -> Result<Vec<crate::types::Ad>> {
        tracing::debug!("Listing ads for adset: {}", adset_id);
        Ok(vec![])
    }

    pub async fn update_ad(&self, ad_id: &str, request: UpdateAdRequest) -> Result<crate::types::Ad> {
        tracing::info!("Updating ad: {}", ad_id);
        let mut ad = self.get_ad(ad_id).await?.unwrap();

        if let Some(name) = request.name {
            ad.name = name;
        }

        if let Some(status) = request.status {
            ad.status = match status.to_lowercase().as_str() {
                "active" => AdStatus::Active,
                "paused" => AdStatus::Paused,
                "archived" => AdStatus::Archived,
                "deleted" => AdStatus::Deleted,
                _ => AdStatus::Paused,
            };
        }

        if let Some(creative_input) = request.creative {
            let creative_type = match creative_input.creative_type.to_lowercase().as_str() {
                "image" => CreativeType::Image,
                "video" => CreativeType::Video,
                "carousel" => CreativeType::Carousel,
                "collection" => CreativeType::Collection,
                _ => CreativeType::SingleLink,
            };

            ad.creative = AdCreative {
                id: ad.creative.id.clone(),
                name: creative_input.name,
                creative_type,
                title: creative_input.title,
                body: creative_input.body,
                image_url: creative_input.image_url,
                video_url: creative_input.video_url,
                call_to_action: creative_input.call_to_action_type.as_ref().map(|cta| {
                    CallToAction {
                        action_type: cta.clone(),
                        value: serde_json::json!({
                            "link": creative_input.link.unwrap_or_default()
                        }),
                    }
                }),
                page_id: creative_input.page_id,
                product_set_id: creative_input.product_set_id,
            };
        }

        Ok(ad)
    }

    pub async fn preview_ad(&self, ad_id: &str) -> Result<String> {
        tracing::info!("Generating preview for ad: {}", ad_id);
        if let Some(ad) = self.get_ad(ad_id).await? {
            Ok(format!(
                "Preview: {} - {} - {}",
                ad.name,
                ad.creative.title.as_deref().unwrap_or("No title"),
                ad.creative.body.as_deref().unwrap_or("No body")
            ))
        } else {
            anyhow::bail!("Ad not found: {}", ad_id)
        }
    }

    pub async fn generate_ad_copy(&self, product_description: &str, tone: &str) -> Result<AdCopyResult> {
        tracing::info!("Generating ad copy for: {}", product_description);

        Ok(AdCopyResult {
            headlines: vec![
                "Khám phá ngay!".to_string(),
                "Mua ngay hôm nay!".to_string(),
                "Giảm giá sốc!".to_string(),
            ],
            descriptions: vec![
                format!("{} - Chất lượng cao, giá tốt nhất!", product_description),
                format!("{} - Đặt hàng ngay hôm nay!", product_description),
            ],
            call_to_action: "MUA NGAY".to_string(),
            target_audience: "Người mua sắm online tại Việt Nam".to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdCopyResult {
    pub headlines: Vec<String>,
    pub descriptions: Vec<String>,
    pub call_to_action: String,
    pub target_audience: String,
}
