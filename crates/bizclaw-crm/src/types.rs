//! # CRM Types - Customer Relationship Management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub crm_name: Option<String>,
    pub platform_name: String,
    pub platform: Platform,
    pub platform_id: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub pipeline_status: PipelineStatus,
    pub tags: Vec<String>,
    pub notes: String,
    pub custom_fields: HashMap<String, String>,
    pub source: ContactSource,
    pub assigned_to: Option<String>,
    pub score: u32,
    pub total_spent: f64,
    pub message_count: u64,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContactRequest {
    pub name: String,
    pub channel: String,
    pub channel_id: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Zalo,
    Facebook,
    Instagram,
    #[serde(rename = "telegram")]
    Telegram,
    #[serde(rename = "shopee")]
    Shopee,
    #[serde(rename = "lazada")]
    Lazada,
    #[serde(rename = "website")]
    Website,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    New,
    Contacted,
    Interested,
    Converted,
    Lost,
}

impl PipelineStatus {
    pub fn next(&self) -> Option<Self> {
        match self {
            PipelineStatus::New => Some(PipelineStatus::Contacted),
            PipelineStatus::Contacted => Some(PipelineStatus::Interested),
            PipelineStatus::Interested => Some(PipelineStatus::Converted),
            PipelineStatus::Converted | PipelineStatus::Lost => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PipelineStatus::New => "🆕 Mới",
            PipelineStatus::Contacted => "📞 Đã liên hệ",
            PipelineStatus::Interested => "🤔 Quan tâm",
            PipelineStatus::Converted => "✅ Chuyển đổi",
            PipelineStatus::Lost => "❌ Mất",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContactSource {
    Organic,
    Ad,
    Referral,
    Website,
    Chat,
    Phone,
    Walkin,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub contact_id: String,
    pub platform: Platform,
    pub thread_id: String,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: u32,
    pub is_typing: bool,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub tab: ConversationTab,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationTab {
    All,
    Unread,
    Unanswered,
    Important,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_type: SenderType,
    pub content: String,
    pub message_type: MessageType,
    pub attachments: Vec<Attachment>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SenderType {
    Customer,
    Agent,
    Bot,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    Image,
    Video,
    Audio,
    File,
    Sticker,
    Location,
    Transfer,
    Call,
    Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub url: String,
    pub mime_type: String,
    pub filename: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    pub id: String,
    pub name: String,
    pub content: String,
    pub variables: Vec<TemplateVariable>,
    pub shortcut: String,
    pub category: String,
    pub use_count: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub example: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Appointment {
    pub id: String,
    pub contact_id: String,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_at: DateTime<Utc>,
    pub remind_at: Option<DateTime<Utc>>,
    pub status: AppointmentStatus,
    pub assigned_to: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppointmentStatus {
    Scheduled,
    Completed,
    Cancelled,
    NoShow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: TeamRole,
    pub avatar: Option<String>,
    pub is_online: bool,
    pub conversation_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Owner,
    Admin,
    Manager,
    Agent,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRMDashboard {
    pub total_contacts: u64,
    pub new_contacts_today: u64,
    pub active_conversations: u64,
    pub unread_messages: u64,
    pub pipeline_counts: HashMap<PipelineStatus, u64>,
    pub avg_response_time_minutes: f64,
    pub conversion_rate: f64,
    pub top_sources: Vec<(ContactSource, u64)>,
    pub messages_today: u64,
    pub appointments_today: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIReplySuggestion {
    pub id: String,
    pub message_id: String,
    pub content: String,
    pub confidence: f32,
    pub tone: ReplyTone,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplyTone {
    Friendly,
    Professional,
    Urgent,
    Followup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub id: String,
    pub name: String,
    pub event_type: WorkflowEvent,
    pub conditions: Vec<WorkflowCondition>,
    pub actions: Vec<WorkflowAction>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEvent {
    NewContact,
    NewMessage,
    KeywordDetected,
    PipelineChange,
    AppointmentReminder,
    TimeDelay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCondition {
    pub field: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAction {
    pub action_type: WorkflowActionType,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionType {
    SendMessage,
    UpdatePipeline,
    AddTag,
    AssignAgent,
    SendTemplate,
    AiReply,
}
