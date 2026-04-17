// CRM Routes - OmniChannel Customer Management

use axum::{
    Json,
    extract::{Path, Query, State},
};
use bizclaw_crm::{
    Contact, ContactSource, ConversationTab, CreateContactRequest, PipelineStatus, Platform,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::AppState;

#[derive(Serialize)]
pub struct ContactResponse {
    pub id: String,
    pub primary_name: String,
    pub channels: Vec<ChannelInfo>,
    pub pipeline: String,
    pub pipeline_label: String,
    pub unread_count: u32,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub tags: Vec<String>,
    pub score: u32,
    pub total_spent: f64,
    pub message_count: u64,
    pub last_message_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ChannelInfo {
    pub channel: String,
    pub phone: Vec<String>,
    pub email: Vec<String>,
}

#[derive(Serialize)]
pub struct ConversationResponse {
    pub id: String,
    pub contact_id: String,
    pub platform: String,
    pub last_message: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: u32,
    pub is_pinned: bool,
    pub tab: String,
    pub contact_name: String,
}

#[derive(Serialize)]
pub struct DashboardResponse {
    pub total_contacts: u64,
    pub new_contacts_today: u64,
    pub active_conversations: u64,
    pub unread_messages: u64,
    pub pipeline_counts: Vec<PipelineCount>,
    pub conversion_rate: f64,
    pub messages_today: u64,
    pub appointments_today: u64,
}

#[derive(Serialize)]
pub struct PipelineCount {
    pub status: String,
    pub label: String,
    pub count: u64,
}

#[derive(Deserialize)]
pub struct ListContactsQuery {
    pub search: Option<String>,
    pub pipeline: Option<String>,
    pub tag: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ListConversationsQuery {
    pub tab: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePipelineRequest {
    pub status: String,
}

fn contact_to_response(contact: &Contact) -> ContactResponse {
    let pipeline_label = contact.pipeline_status.label().to_string();
    let pipeline_name = match contact.pipeline_status {
        PipelineStatus::New => "new",
        PipelineStatus::Contacted => "contacted",
        PipelineStatus::Interested => "interested",
        PipelineStatus::Converted => "converted",
        PipelineStatus::Lost => "lost",
    };

    ContactResponse {
        id: contact.id.clone(),
        primary_name: contact
            .crm_name
            .clone()
            .unwrap_or_else(|| contact.platform_name.clone()),
        channels: vec![ChannelInfo {
            channel: format!("{:?}", contact.platform).to_lowercase(),
            phone: contact
                .phone
                .as_ref()
                .map(|p| vec![p.clone()])
                .unwrap_or_default(),
            email: contact
                .email
                .as_ref()
                .map(|e| vec![e.clone()])
                .unwrap_or_default(),
        }],
        pipeline: pipeline_name.to_string(),
        pipeline_label,
        unread_count: 0,
        phone: contact.phone.clone(),
        email: contact.email.clone(),
        tags: contact.tags.clone(),
        score: contact.score,
        total_spent: contact.total_spent,
        message_count: contact.message_count,
        last_message_at: contact.last_message_at.map(|dt| dt.to_rfc3339()),
        created_at: contact.created_at.to_rfc3339(),
    }
}

pub async fn list_contacts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListContactsQuery>,
) -> Json<Vec<ContactResponse>> {
    let limit = query.limit.unwrap_or(50);
    let page = query.page.unwrap_or(0);
    let offset = page * limit;

    let contacts = state.crm.contacts.read().await;
    let mut results: Vec<_> = contacts.values().collect();

    if let Some(ref search) = query.search {
        let search_lower = search.to_lowercase();
        results.retain(|c| {
            c.crm_name
                .as_ref()
                .map(|n| n.to_lowercase().contains(&search_lower))
                .unwrap_or(false)
                || c.platform_name.to_lowercase().contains(&search_lower)
                || c.phone
                    .as_ref()
                    .map(|p| p.contains(search))
                    .unwrap_or(false)
                || c.email
                    .as_ref()
                    .map(|e| e.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
        });
    }

    if let Some(ref pipeline) = query.pipeline {
        results.retain(|c| match c.pipeline_status {
            PipelineStatus::New => pipeline == "new",
            PipelineStatus::Contacted => pipeline == "contacted",
            PipelineStatus::Interested => pipeline == "interested",
            PipelineStatus::Converted => pipeline == "converted",
            PipelineStatus::Lost => pipeline == "lost",
        });
    }

    if let Some(ref tag) = query.tag {
        results.retain(|c| c.tags.iter().any(|t| t == tag));
    }

    results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    let paginated: Vec<ContactResponse> = results
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(contact_to_response)
        .collect();

    Json(paginated)
}

pub async fn create_contact(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateContactRequest>,
) -> Json<serde_json::Value> {
    let platform = match req.channel.to_lowercase().as_str() {
        "zalo" => Platform::Zalo,
        "facebook" => Platform::Facebook,
        "instagram" => Platform::Instagram,
        "telegram" => Platform::Telegram,
        _ => Platform::Other,
    };

    let contact = Contact {
        id: uuid::Uuid::new_v4().to_string(),
        crm_name: Some(req.name.clone()),
        platform_name: req.name.clone(),
        platform,
        platform_id: req.channel_id.clone(),
        phone: req.phone.clone(),
        email: req.email.clone(),
        avatar: None,
        pipeline_status: PipelineStatus::New,
        tags: vec![],
        notes: String::new(),
        custom_fields: std::collections::HashMap::new(),
        source: ContactSource::Chat,
        assigned_to: None,
        score: 50,
        total_spent: 0.0,
        message_count: 0,
        last_message_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    match state.crm.create_contact(contact).await {
        Ok(id) => Json(serde_json::json!({
            "ok": true,
            "id": id,
            "message": "Contact created successfully"
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

pub async fn get_contact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if let Some(contact) = state.crm.get_contact(&id).await {
        Json(serde_json::json!({
            "ok": true,
            "contact": contact_to_response(&contact)
        }))
    } else {
        Json(serde_json::json!({
            "ok": false,
            "error": "Contact not found"
        }))
    }
}

pub async fn update_contact(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateContactRequest>,
) -> Json<serde_json::Value> {
    if let Some(mut contact) = state.crm.get_contact(&id).await {
        if !req.name.is_empty() {
            contact.crm_name = Some(req.name.clone());
            contact.platform_name = req.name;
        }
        if req.phone.is_some() {
            contact.phone = req.phone;
        }
        if req.email.is_some() {
            contact.email = req.email;
        }
        contact.updated_at = chrono::Utc::now();

        state.crm.contacts.write().await.insert(id.clone(), contact);

        Json(serde_json::json!({
            "ok": true,
            "message": "Contact updated successfully"
        }))
    } else {
        Json(serde_json::json!({
            "ok": false,
            "error": "Contact not found"
        }))
    }
}

pub async fn update_pipeline(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePipelineRequest>,
) -> Json<serde_json::Value> {
    let status = match req.status.to_lowercase().as_str() {
        "new" => PipelineStatus::New,
        "contacted" => PipelineStatus::Contacted,
        "interested" => PipelineStatus::Interested,
        "converted" => PipelineStatus::Converted,
        "lost" => PipelineStatus::Lost,
        _ => {
            return Json(serde_json::json!({
                "ok": false,
                "error": "Invalid pipeline status"
            }));
        }
    };

    if let Err(e) = state.crm.update_pipeline(&id, status).await {
        return Json(serde_json::json!({
            "ok": false,
            "error": e.to_string()
        }));
    }

    Json(serde_json::json!({
        "ok": true,
        "message": "Pipeline updated successfully"
    }))
}

pub async fn get_interactions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let conversations = state.crm.conversations.read().await;
    let interactions: Vec<serde_json::Value> = conversations
        .values()
        .filter(|c| c.contact_id == id)
        .flat_map(|conv| &conv.messages)
        .map(|msg| {
            serde_json::json!({
                "id": msg.id,
                "sender_type": format!("{:?}", msg.sender_type).to_lowercase(),
                "content": msg.content,
                "created_at": msg.created_at.to_rfc3339(),
                "is_read": msg.is_read,
            })
        })
        .collect();

    Json(serde_json::json!({
        "ok": true,
        "interactions": interactions,
    }))
}

pub async fn create_interaction(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = uuid::Uuid::new_v4().to_string();
    Json(serde_json::json!({
        "ok": true,
        "id": id,
        "message": "Interaction logged"
    }))
}

pub async fn list_conversations(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListConversationsQuery>,
) -> Json<Vec<ConversationResponse>> {
    let tab = query
        .tab
        .as_ref()
        .and_then(|t| match t.to_lowercase().as_str() {
            "unread" => Some(ConversationTab::Unread),
            "important" => Some(ConversationTab::Important),
            "other" => Some(ConversationTab::Other),
            _ => Some(ConversationTab::All),
        });

    let conversations = state.crm.get_conversations(tab).await;
    let contacts = state.crm.contacts.read().await;

    let results: Vec<ConversationResponse> = conversations
        .into_iter()
        .map(|conv| {
            let contact_name = contacts
                .get(&conv.contact_id)
                .map(|c| {
                    c.crm_name
                        .clone()
                        .unwrap_or_else(|| c.platform_name.clone())
                })
                .unwrap_or_else(|| "Unknown".to_string());

            ConversationResponse {
                id: conv.id,
                contact_id: conv.contact_id,
                platform: format!("{:?}", conv.platform).to_lowercase(),
                last_message: conv.last_message,
                last_message_at: conv.last_message_at.map(|dt| dt.to_rfc3339()),
                unread_count: conv.unread_count,
                is_pinned: conv.is_pinned,
                tab: format!("{:?}", conv.tab).to_lowercase(),
                contact_name,
            }
        })
        .collect();

    Json(results)
}

pub async fn get_conversation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let conversations = state.crm.conversations.read().await;
    if let Some(conv) = conversations.get(&id) {
        Json(serde_json::json!({
            "ok": true,
            "conversation": {
                "id": conv.id,
                "contact_id": conv.contact_id,
                "platform": format!("{:?}", conv.platform).to_lowercase(),
                "messages": conv.messages,
                "unread_count": conv.unread_count,
            }
        }))
    } else {
        Json(serde_json::json!({
            "ok": false,
            "error": "Conversation not found"
        }))
    }
}

pub async fn mark_read(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.crm.mark_read(&id).await {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "message": "Marked as read"
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e.to_string()
        })),
    }
}

pub async fn get_dashboard(State(state): State<Arc<AppState>>) -> Json<DashboardResponse> {
    let dashboard = state.crm.get_dashboard().await;

    let pipeline_counts: Vec<PipelineCount> = vec![
        PipelineCount {
            status: "new".to_string(),
            label: "🆕 Mới".to_string(),
            count: *dashboard
                .pipeline_counts
                .get(&PipelineStatus::New)
                .unwrap_or(&0),
        },
        PipelineCount {
            status: "contacted".to_string(),
            label: "📞 Đã liên hệ".to_string(),
            count: *dashboard
                .pipeline_counts
                .get(&PipelineStatus::Contacted)
                .unwrap_or(&0),
        },
        PipelineCount {
            status: "interested".to_string(),
            label: "🤔 Quan tâm".to_string(),
            count: *dashboard
                .pipeline_counts
                .get(&PipelineStatus::Interested)
                .unwrap_or(&0),
        },
        PipelineCount {
            status: "converted".to_string(),
            label: "✅ Chuyển đổi".to_string(),
            count: *dashboard
                .pipeline_counts
                .get(&PipelineStatus::Converted)
                .unwrap_or(&0),
        },
        PipelineCount {
            status: "lost".to_string(),
            label: "❌ Mất".to_string(),
            count: *dashboard
                .pipeline_counts
                .get(&PipelineStatus::Lost)
                .unwrap_or(&0),
        },
    ];

    Json(DashboardResponse {
        total_contacts: dashboard.total_contacts,
        new_contacts_today: dashboard.new_contacts_today,
        active_conversations: dashboard.active_conversations,
        unread_messages: dashboard.unread_messages,
        pipeline_counts,
        conversion_rate: dashboard.conversion_rate,
        messages_today: dashboard.messages_today,
        appointments_today: dashboard.appointments_today,
    })
}
