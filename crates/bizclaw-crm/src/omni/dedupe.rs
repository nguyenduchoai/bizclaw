//! # Contact Deduplication Engine
//!
//! Fuzzy matching + deterministic matching for merging contacts from multiple channels

use crate::omni::{Channel, ChannelContact, UnifiedContact};
use chrono::Utc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::HashMap;

pub struct DedupeEngine {
    matcher: SkimMatcherV2,
    thresholds: DedupeThresholds,
}

#[derive(Debug, Clone)]
pub struct DedupeThresholds {
    pub phone_exact: f32,
    pub email_exact: f32,
    pub name_fuzzy: f32,
    pub composite: f32,
}

impl Default for DedupeThresholds {
    fn default() -> Self {
        Self {
            phone_exact: 100.0,
            email_exact: 100.0,
            name_fuzzy: 80.0,
            composite: 70.0,
        }
    }
}

impl DedupeEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            thresholds: DedupeThresholds::default(),
        }
    }

    pub fn with_thresholds(thresholds: DedupeThresholds) -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            thresholds,
        }
    }

    pub fn find_duplicates(&self, contacts: &[ChannelContact]) -> Vec<DedupeMatch> {
        let mut matches = Vec::new();
        let n = contacts.len();

        for i in 0..n {
            for j in (i + 1)..n {
                if let Some(score) = self.calculate_match_score(&contacts[i], &contacts[j]) {
                    if score >= self.thresholds.composite {
                        matches.push(DedupeMatch {
                            contact_a: i,
                            contact_b: j,
                            score,
                            match_type: self.get_match_type(&contacts[i], &contacts[j]),
                        });
                    }
                }
            }
        }

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches
    }

    pub fn calculate_match_score(&self, a: &ChannelContact, b: &ChannelContact) -> Option<f32> {
        let mut scores = Vec::new();

        for phone_a in &a.phone {
            for phone_b in &b.phone {
                if phone_a == phone_b && !phone_a.is_empty() {
                    scores.push(self.thresholds.phone_exact);
                }
            }
        }

        for email_a in &a.email {
            for email_b in &b.email {
                if email_a.to_lowercase() == email_b.to_lowercase() && !email_a.is_empty() {
                    scores.push(self.thresholds.email_exact);
                }
            }
        }

        let name_score = self
            .matcher
            .fuzzy_match(&a.display_name, &b.display_name)
            .unwrap_or(0) as f32;
        if name_score >= self.thresholds.name_fuzzy {
            scores.push(name_score);
        }

        if a.channel == b.channel && a.channel_contact_id == b.channel_contact_id {
            scores.push(100.0);
        }

        if scores.is_empty() {
            None
        } else {
            let total: f32 = scores.iter().sum();
            let count = scores.len() as f32;
            Some(total / count)
        }
    }

    fn get_match_type(&self, a: &ChannelContact, b: &ChannelContact) -> MatchType {
        if a.phone.iter().any(|p| !p.is_empty() && b.phone.contains(p)) {
            MatchType::Phone
        } else if a.email.iter().any(|e| {
            !e.is_empty()
                && b.email
                    .iter()
                    .any(|eb| e.to_lowercase() == eb.to_lowercase())
        }) {
            MatchType::Email
        } else if self
            .matcher
            .fuzzy_match(&a.display_name, &b.display_name)
            .unwrap_or(0)
            >= 80
        {
            MatchType::Name
        } else {
            MatchType::Composite
        }
    }

    pub fn merge_contacts(&self, contacts: Vec<ChannelContact>) -> UnifiedContact {
        let primary = contacts.first().cloned().unwrap_or_else(|| ChannelContact {
            channel: Channel::Other("unknown".to_string()),
            channel_contact_id: uuid::Uuid::new_v4().to_string(),
            display_name: "Unknown".to_string(),
            avatar: None,
            phone: vec![],
            email: vec![],
            address: None,
            dob: None,
            gender: None,
            is_verified: false,
            metadata: HashMap::new(),
            linked_at: Utc::now(),
        });

        let mut all_phones = Vec::new();
        let mut all_emails = Vec::new();
        let mut addresses = Vec::new();
        let mut dobs = Vec::new();
        let mut genders = Vec::new();
        let mut verified = false;

        for c in &contacts {
            all_phones.extend(c.phone.clone());
            all_emails.extend(c.email.clone());
            if c.address.is_some() {
                addresses.push(c.address.clone().unwrap());
            }
            if c.dob.is_some() {
                dobs.push(c.dob.clone().unwrap());
            }
            if c.gender.is_some() {
                genders.push(c.gender.clone().unwrap());
            }
            if c.is_verified {
                verified = true;
            }
        }

        let score = if contacts.len() > 1 {
            let mut total = 0.0;
            let mut count = 0;
            for i in 0..contacts.len() {
                for j in (i + 1)..contacts.len() {
                    if let Some(score) = self.calculate_match_score(&contacts[i], &contacts[j]) {
                        total += score;
                        count += 1;
                    }
                }
            }
            if count > 0 {
                total / (count as f32)
            } else {
                100.0
            }
        } else {
            100.0
        };

        UnifiedContact {
            id: uuid::Uuid::new_v4().to_string(),
            primary_name: primary.display_name.clone(),
            channels: contacts,
            merged_ids: vec![],
            merge_confidence: score,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for DedupeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DedupeMatch {
    pub contact_a: usize,
    pub contact_b: usize,
    pub score: f32,
    pub match_type: MatchType,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    Phone,
    Email,
    Name,
    ChannelID,
    Composite,
}
