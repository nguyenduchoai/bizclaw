//! CRM Manager - AI Social CRM Core

use crate::types::*;
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct CRMManager {
    pub contacts: Arc<RwLock<HashMap<String, Contact>>>,
    pub conversations: Arc<RwLock<HashMap<String, Conversation>>>,
    pub templates: Arc<RwLock<HashMap<String, MessageTemplate>>>,
    pub appointments: Arc<RwLock<HashMap<String, Appointment>>>,
    pub team_members: Arc<RwLock<HashMap<String, TeamMember>>>,
    pub workflows: Arc<RwLock<Vec<WorkflowTrigger>>>,
}

impl CRMManager {
    pub fn new() -> Self {
        Self {
            contacts: Arc::new(RwLock::new(HashMap::new())),
            conversations: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            appointments: Arc::new(RwLock::new(HashMap::new())),
            team_members: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_contact(&self, contact: Contact) -> Result<String> {
        let id = contact.id.clone();
        self.contacts.write().await.insert(id.clone(), contact);
        info!("Created contact: {}", id);
        Ok(id)
    }

    pub async fn get_contact(&self, id: &str) -> Option<Contact> {
        self.contacts.read().await.get(id).cloned()
    }

    pub async fn update_pipeline(&self, contact_id: &str, status: PipelineStatus) -> Result<()> {
        let mut contacts = self.contacts.write().await;
        if let Some(contact) = contacts.get_mut(contact_id) {
            contact.pipeline_status = status;
            contact.updated_at = Utc::now();
        }
        Ok(())
    }

    pub async fn search_contacts(&self, query: &str) -> Vec<Contact> {
        let query_lower = query.to_lowercase();
        self.contacts
            .read()
            .await
            .values()
            .filter(|c| {
                c.crm_name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
                    || c.platform_name.to_lowercase().contains(&query_lower)
                    || c.phone.as_ref().map(|p| p.contains(query)).unwrap_or(false)
                    || c.email
                        .as_ref()
                        .map(|e| e.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || c.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    pub async fn get_conversations(&self, tab: Option<ConversationTab>) -> Vec<Conversation> {
        let convs = self.conversations.read().await;
        match tab {
            Some(t) => convs
                .values()
                .filter(|c| c.tab == t || (t == ConversationTab::All && !c.is_archived))
                .cloned()
                .collect(),
            None => convs.values().filter(|c| !c.is_archived).cloned().collect(),
        }
    }

    pub async fn mark_read(&self, conv_id: &str) -> Result<()> {
        let mut convs = self.conversations.write().await;
        if let Some(conv) = convs.get_mut(conv_id) {
            conv.unread_count = 0;
            for msg in &mut conv.messages {
                msg.is_read = true;
            }
        }
        Ok(())
    }

    pub async fn render_template(&self, shortcut: &str, contact: &Contact) -> Option<String> {
        let templates = self.templates.read().await;
        let tmpl = templates.get(shortcut)?;

        let mut content = tmpl.content.clone();

        content = content.replace(
            "{name}",
            contact
                .crm_name
                .as_deref()
                .unwrap_or(&contact.platform_name),
        );
        content = content.replace("{phone}", contact.phone.as_deref().unwrap_or(""));
        content = content.replace("{status}", contact.pipeline_status.label());
        content = content.replace("{date}", &Utc::now().format("%d/%m/%Y").to_string());

        Some(content)
    }

    pub async fn suggest_replies(&self, conversation_id: &str) -> Result<Vec<AIReplySuggestion>> {
        let convs = self.conversations.read().await;
        let conv = convs
            .get(conversation_id)
            .context("Conversation not found")?;

        let last_customer_msg = conv
            .messages
            .iter()
            .rev()
            .find(|m| m.sender_type == SenderType::Customer);

        let mut suggestions = Vec::new();

        if let Some(msg) = last_customer_msg {
            let content_lower = msg.content.to_lowercase();

            if content_lower.contains("giá") || content_lower.contains("bao nhiêu") {
                suggestions.push(AIReplySuggestion {
                    id: uuid::Uuid::new_v4().to_string(),
                    message_id: msg.id.clone(),
                    content: "Dạ em xin gửi anh/chị bảng giá ạ".to_string(),
                    confidence: 0.92,
                    tone: ReplyTone::Professional,
                });
            }

            if content_lower.contains("mua") || content_lower.contains("đặt") {
                suggestions.push(AIReplySuggestion {
                    id: uuid::Uuid::new_v4().to_string(),
                    message_id: msg.id.clone(),
                    content: "Dạ em cảm ơn anh/chị đã quan tâm ạ. Để em tư vấn chi tiết hơn ạ!"
                        .to_string(),
                    confidence: 0.88,
                    tone: ReplyTone::Friendly,
                });
            }

            suggestions.push(AIReplySuggestion {
                id: uuid::Uuid::new_v4().to_string(),
                message_id: msg.id.clone(),
                content: "Dạ em đã tiếp nhận thông tin và sẽ phản hồi sớm nhất có thể ạ!"
                    .to_string(),
                confidence: 0.85,
                tone: ReplyTone::Friendly,
            });
        }

        Ok(suggestions)
    }

    pub async fn get_dashboard(&self) -> CRMDashboard {
        let contacts = self.contacts.read().await;
        let conversations = self.conversations.read().await;
        let appointments = self.appointments.read().await;

        let today = Utc::now().date_naive();
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();

        let mut pipeline_counts: HashMap<PipelineStatus, u64> = HashMap::new();
        for status in [
            PipelineStatus::New,
            PipelineStatus::Contacted,
            PipelineStatus::Interested,
            PipelineStatus::Converted,
            PipelineStatus::Lost,
        ] {
            pipeline_counts.insert(status, 0);
        }

        for contact in contacts.values() {
            *pipeline_counts
                .entry(contact.pipeline_status.clone())
                .or_insert(0) += 1;
        }

        let new_contacts_today = contacts
            .values()
            .filter(|c| c.created_at >= today_start)
            .count() as u64;

        let unread_messages = conversations.values().map(|c| c.unread_count as u64).sum();

        let active_conversations = conversations
            .values()
            .filter(|c| !c.is_archived && c.unread_count > 0)
            .count() as u64;

        let messages_today = conversations
            .values()
            .filter(|c| c.last_message_at.map(|t| t >= today_start).unwrap_or(false))
            .count() as u64;

        let appointments_today = appointments
            .values()
            .filter(|a| {
                a.scheduled_at >= today_start && a.scheduled_at < today_start + Duration::days(1)
            })
            .count() as u64;

        let converted = *pipeline_counts
            .get(&PipelineStatus::Converted)
            .unwrap_or(&0) as f64;
        let total = contacts.len() as f64;
        let conversion_rate = if total > 0.0 {
            converted / total * 100.0
        } else {
            0.0
        };

        let mut source_counts: HashMap<ContactSource, u64> = HashMap::new();
        for contact in contacts.values() {
            *source_counts.entry(contact.source.clone()).or_insert(0) += 1;
        }
        let mut top_sources: Vec<_> = source_counts.into_iter().collect();
        top_sources.sort_by(|a, b| b.1.cmp(&a.1));
        let top_sources = top_sources.into_iter().take(5).collect();

        CRMDashboard {
            total_contacts: contacts.len() as u64,
            new_contacts_today,
            active_conversations,
            unread_messages,
            pipeline_counts,
            avg_response_time_minutes: 15.0,
            conversion_rate,
            top_sources,
            messages_today,
            appointments_today,
        }
    }
}

impl Default for CRMManager {
    fn default() -> Self {
        Self::new()
    }
}
