use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use serde_json::json;

use crate::campaigns::{Campaign, CampaignManager};
use crate::lead::{Lead, LeadStatus};
use crate::sequences::Sequence;
use crate::templates::TemplateEngine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachMessage {
    pub intent: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl OutreachResponse {
    pub fn success(data: impl Serialize) -> Self {
        Self {
            success: true,
            data: serde_json::to_value(data).unwrap_or(json!({})),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: json!({}),
            error: Some(msg.to_string()),
        }
    }
}

pub struct OutreachAgent {
    campaign_manager: Arc<RwLock<CampaignManager>>,
    template_engine: Arc<TemplateEngine>,
}

impl OutreachAgent {
    pub fn new() -> Self {
        Self {
            campaign_manager: Arc::new(RwLock::new(CampaignManager::new())),
            template_engine: Arc::new(TemplateEngine::new()),
        }
    }

    pub async fn create_campaign(&self, name: &str, leads: Vec<Lead>) -> Result<Campaign> {
        info!("Creating campaign: {}", name);
        let mut manager = self.campaign_manager.write().await;
        manager.create_campaign(name, leads).await
    }

    pub async fn add_lead(&self, campaign_id: &str, lead: Lead) -> Result<()> {
        let mut manager = self.campaign_manager.write().await;
        manager.add_lead(campaign_id, lead).await
    }

    pub async fn send_sequence(&self, campaign_id: &str, sequence: Sequence) -> Result<()> {
        info!("Starting sequence for campaign: {}", campaign_id);
        let mut manager = self.campaign_manager.write().await;
        manager.send_sequence(campaign_id, sequence).await
    }

    pub async fn get_lead_status(&self, lead_id: &str) -> Result<Option<LeadStatus>> {
        let manager = self.campaign_manager.read().await;
        Ok(manager.get_lead_status(lead_id).await)
    }

    pub async fn process(&self, message: OutreachMessage) -> Result<OutreachResponse> {
        debug!("Processing outreach message: {:?}", message.intent);

        match message.intent.as_str() {
            "create_campaign" => {
                let name = message.payload.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled");
                let leads: Vec<Lead> = message.payload.get("leads")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let campaign = self.create_campaign(name, leads).await?;
                Ok(OutreachResponse::success(campaign))
            }
            "send_cold_email" => {
                let lead_id = message.payload.get("lead_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let template_id = message.payload.get("template_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("cold_email_basic");
                let personalized = self.template_engine.personalize(template_id, lead_id)?;
                Ok(OutreachResponse::success(json!({
                    "status": "sent",
                    "personalized_content": personalized
                })))
            }
            "send_zalo_message" => {
                let phone = message.payload.get("phone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let message_text = message.payload.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(OutreachResponse::success(json!({
                    "channel": "zalo",
                    "recipient": phone,
                    "message": message_text,
                    "status": "queued"
                })))
            }
            "track_response" => {
                let lead_id = message.payload.get("lead_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let status = self.get_lead_status(lead_id).await?;
                Ok(OutreachResponse::success(status))
            }
            _ => Ok(OutreachResponse::error("Unknown intent")),
        }
    }
}

impl Default for OutreachAgent {
    fn default() -> Self {
        Self::new()
    }
}
