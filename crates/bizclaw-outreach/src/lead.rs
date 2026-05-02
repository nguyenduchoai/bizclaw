use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub zalo_id: Option<String>,
    pub linkedin_url: Option<String>,
    pub company: Option<String>,
    pub position: Option<String>,
    pub source: LeadSource,
    pub status: LeadStatus,
    pub score: i32,
    pub tags: Vec<String>,
    pub notes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_contacted: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LeadStatus {
    New,
    Contacted,
    Qualified,
    Negotiation,
    Won,
    Lost,
    NotInterested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeadSource {
    Website,
    Zalo,
    Facebook,
    LinkedIn,
    Referral,
    ColdCall,
    Event,
    Other,
}

impl Lead {
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            email: None,
            phone: None,
            zalo_id: None,
            linkedin_url: None,
            company: None,
            position: None,
            source: LeadSource::Other,
            status: LeadStatus::New,
            score: 0,
            tags: vec![],
            notes: vec![],
            created_at: now,
            updated_at: now,
            last_contacted: None,
        }
    }

    pub fn with_email(mut self, email: &str) -> Self {
        self.email = Some(email.to_string());
        self.score += 10;
        self
    }

    pub fn with_phone(mut self, phone: &str) -> Self {
        self.phone = Some(phone.to_string());
        self.score += 15;
        self
    }

    pub fn with_zalo(mut self, zalo_id: &str) -> Self {
        self.zalo_id = Some(zalo_id.to_string());
        self.score += 20;
        self
    }

    pub fn with_company(mut self, company: &str, position: &str) -> Self {
        self.company = Some(company.to_string());
        self.position = Some(position.to_string());
        self.score += 25;
        self
    }

    pub fn qualify(&mut self) {
        if self.status == LeadStatus::Contacted && self.score >= 50 {
            self.status = LeadStatus::Qualified;
            self.updated_at = Utc::now();
        }
    }
}
