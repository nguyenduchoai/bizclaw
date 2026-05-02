use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::lead::{Lead, LeadStatus};
use crate::sequences::Sequence;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: CampaignStatus,
    pub leads: Vec<String>,
    pub sequence_id: Option<String>,
    pub stats: CampaignStats,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignStats {
    pub total_leads: usize,
    pub contacted: usize,
    pub replied: usize,
    pub qualified: usize,
    pub converted: usize,
    pub open_rate: f32,
    pub click_rate: f32,
    pub conversion_rate: f32,
}

pub struct CampaignManager {
    campaigns: HashMap<String, Campaign>,
    lead_statuses: HashMap<String, LeadStatus>,
}

impl CampaignManager {
    pub fn new() -> Self {
        Self {
            campaigns: HashMap::new(),
            lead_statuses: HashMap::new(),
        }
    }

    pub async fn create_campaign(&mut self, name: &str, leads: Vec<Lead>) -> Result<Campaign> {
        let campaign_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let lead_ids: Vec<String> = leads.iter().map(|l| l.id.clone()).collect();
        for lead in &leads {
            self.lead_statuses.insert(lead.id.clone(), lead.status);
        }

        let campaign = Campaign {
            id: campaign_id.clone(),
            name: name.to_string(),
            description: None,
            status: CampaignStatus::Draft,
            leads: lead_ids,
            sequence_id: None,
            stats: CampaignStats {
                total_leads: leads.len(),
                ..Default::default()
            },
            created_at: now,
            updated_at: now,
        };

        self.campaigns.insert(campaign_id, campaign.clone());
        Ok(campaign)
    }

    pub async fn add_lead(&mut self, campaign_id: &str, lead: Lead) -> Result<()> {
        if let Some(campaign) = self.campaigns.get_mut(campaign_id) {
            campaign.leads.push(lead.id.clone());
            campaign.stats.total_leads += 1;
            campaign.updated_at = Utc::now();
            self.lead_statuses.insert(lead.id, LeadStatus::New);
        }
        Ok(())
    }

    pub async fn send_sequence(&mut self, campaign_id: &str, sequence: Sequence) -> Result<()> {
        if let Some(campaign) = self.campaigns.get_mut(campaign_id) {
            campaign.status = CampaignStatus::Active;
            campaign.sequence_id = Some(sequence.id.clone());
            campaign.updated_at = Utc::now();
        }
        Ok(())
    }

    pub async fn get_lead_status(&self, lead_id: &str) -> Option<LeadStatus> {
        self.lead_statuses.get(lead_id).copied()
    }

    pub async fn update_lead_status(&mut self, lead_id: &str, status: LeadStatus) {
        self.lead_statuses.insert(lead_id.to_string(), status);
    }
}

impl Default for CampaignManager {
    fn default() -> Self {
        Self::new()
    }
}
