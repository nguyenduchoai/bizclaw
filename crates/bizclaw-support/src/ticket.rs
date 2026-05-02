use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub ticket_number: String,
    pub customer_id: String,
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub customer_email: Option<String>,
    pub channel: TicketChannel,
    pub subject: String,
    pub description: String,
    pub category: TicketCategory,
    pub priority: TicketPriority,
    pub status: TicketStatus,
    pub assigned_to: Option<String>,
    pub messages: Vec<TicketMessage>,
    pub sla: SlaInfo,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TicketChannel {
    Zalo,
    Facebook,
    Website,
    Phone,
    Email,
    InStore,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TicketCategory {
    Order,
    Product,
    Delivery,
    Payment,
    Return,
    Complaint,
    General,
    Technical,
    Billing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TicketPriority {
    Urgent,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    New,
    Open,
    Pending,
    OnHold,
    Solved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketMessage {
    pub id: String,
    pub sender: MessageSender,
    pub sender_name: String,
    pub content: String,
    pub attachments: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageSender {
    Customer,
    Agent,
    System,
    Bot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaInfo {
    pub first_response_due: DateTime<Utc>,
    pub resolution_due: DateTime<Utc>,
    pub first_response_at: Option<DateTime<Utc>>,
    pub breached: bool,
}

impl Ticket {
    pub fn new(customer_id: &str, customer_name: &str, subject: &str, category: TicketCategory) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            ticket_number: Self::generate_number(),
            customer_id: customer_id.to_string(),
            customer_name: customer_name.to_string(),
            customer_phone: None,
            customer_email: None,
            channel: TicketChannel::Website,
            subject: subject.to_string(),
            description: String::new(),
            category,
            priority: TicketPriority::Normal,
            status: TicketStatus::New,
            assigned_to: None,
            messages: vec![],
            sla: SlaInfo {
                first_response_due: now + chrono::Duration::hours(2),
                resolution_due: now + chrono::Duration::hours(24),
                first_response_at: None,
                breached: false,
            },
            tags: vec![],
            created_at: now,
            updated_at: now,
            resolved_at: None,
        }
    }

    fn generate_number() -> String {
        let now = Utc::now();
        let seq = rand::random::<u32>() % 10000;
        format!("TK-{}{:02}{:02}-{:04}", now.format("%Y%m%d"), seq)
    }

    pub fn add_message(&mut self, sender: MessageSender, sender_name: &str, content: &str) {
        if self.messages.is_empty() && sender == MessageSender::Agent {
            self.sla.first_response_at = Some(Utc::now());
        }
        
        self.messages.push(TicketMessage {
            id: Uuid::new_v4().to_string(),
            sender,
            sender_name: sender_name.to_string(),
            content: content.to_string(),
            attachments: vec![],
            created_at: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    pub fn assign(&mut self, agent_id: &str) {
        self.assigned_to = Some(agent_id.to_string());
        self.status = TicketStatus::Open;
        self.updated_at = Utc::now();
    }

    pub fn resolve(&mut self) {
        self.status = TicketStatus::Solved;
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn check_sla_breach(&mut self) {
        let now = Utc::now();
        if self.sla.first_response_at.is_none() && now > self.sla.first_response_due {
            self.sla.breached = true;
        }
    }
}

pub struct TicketManager {
    tickets: std::collections::HashMap<String, Ticket>,
    by_customer: std::collections::HashMap<String, Vec<String>>,
}

impl TicketManager {
    pub fn new() -> Self {
        Self {
            tickets: std::collections::HashMap::new(),
            by_customer: std::collections::HashMap::new(),
        }
    }

    pub async fn create_ticket(&mut self, ticket: Ticket) -> anyhow::Result<Ticket> {
        self.tickets.insert(ticket.id.clone(), ticket.clone());
        self.by_customer
            .entry(ticket.customer_id.clone())
            .or_insert_with(Vec::new)
            .push(ticket.id.clone());
        Ok(ticket)
    }

    pub async fn get_ticket(&self, id: &str) -> Option<&Ticket> {
        self.tickets.get(id)
    }

    pub async fn get_customer_tickets(&self, customer_id: &str) -> Vec<&Ticket> {
        self.by_customer
            .get(customer_id)
            .map(|ids| ids.iter().filter_map(|id| self.tickets.get(id)).collect()
        .unwrap_or_default()
    }

    pub async fn get_pending_tickets(&self) -> Vec<&Ticket> {
        self.tickets.values()
            .filter(|t| matches!(t.status, TicketStatus::New | TicketStatus::Open))
            .collect()
    }
}

impl Default for TicketManager {
    fn default() -> Self {
        Self::new()
    }
}
