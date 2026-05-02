use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use serde_json::json;

use crate::ticket::{Ticket, TicketManager, TicketCategory, TicketPriority, TicketChannel, MessageSender};
use crate::customer::{Customer, CustomerManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportMessage {
    pub intent: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

impl SupportResponse {
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

pub struct SupportAgent {
    ticket_manager: Arc<RwLock<TicketManager>>,
    customer_manager: Arc<RwLock<CustomerManager>>,
}

impl SupportAgent {
    pub fn new() -> Self {
        Self {
            ticket_manager: Arc::new(RwLock::new(TicketManager::new())),
            customer_manager: Arc::new(RwLock::new(CustomerManager::new())),
        }
    }

    pub async fn create_ticket(
        &self,
        customer_id: &str,
        customer_name: &str,
        subject: &str,
        category: TicketCategory,
    ) -> Result<Ticket> {
        info!("Creating ticket for customer {}", customer_id);
        let ticket = Ticket::new(customer_id, customer_name, subject, category);
        let mut manager = self.ticket_manager.write().await;
        manager.create_ticket(ticket).await
    }

    pub async fn add_reply(&self, ticket_id: &str, sender: MessageSender, sender_name: &str, content: &str) -> Result<()> {
        let mut manager = self.ticket_manager.write().await;
        if let Some(ticket) = manager.tickets.get_mut(ticket_id) {
            ticket.add_message(sender, sender_name, content);
        }
        Ok(())
    }

    pub async fn get_customer_tickets(&self, customer_id: &str) -> Vec<serde_json::Value> {
        let manager = self.ticket_manager.read().await;
        manager.get_customer_tickets(customer_id)
            .await
            .iter()
            .map(|t| serde_json::to_value(t).unwrap_or(json!({})))
            .collect()
    }

    pub async fn create_customer(&self, customer: Customer) -> Result<Customer> {
        let mut manager = self.customer_manager.write().await;
        manager.create_customer(customer).await
    }

    pub async fn find_customer(&self, phone: Option<&str>, zalo: Option<&str>) -> Option<Customer> {
        let manager = self.customer_manager.read().await;
        if let Some(p) = phone {
            manager.find_by_phone(p).await.cloned()
        } else if let Some(z) = zalo {
            manager.find_by_zalo(z).await.cloned()
        } else {
            None
        }
    }

    pub async fn process(&self, message: SupportMessage) -> Result<SupportResponse> {
        debug!("Processing support message: {:?}", message.intent);

        match message.intent.as_str() {
            "create_ticket" => {
                let customer_id = message.payload.get("customer_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let customer_name = message.payload.get("customer_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Customer");
                let subject = message.payload.get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Support Request");
                let category_str = message.payload.get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("general");
                
                let category = match category_str {
                    "order" => TicketCategory::Order,
                    "product" => TicketCategory::Product,
                    "delivery" => TicketCategory::Delivery,
                    "payment" => TicketCategory::Payment,
                    "return" => TicketCategory::Return,
                    "complaint" => TicketCategory::Complaint,
                    "billing" => TicketCategory::Billing,
                    "technical" => TicketCategory::Technical,
                    _ => TicketCategory::General,
                };
                
                let ticket = self.create_ticket(customer_id, customer_name, subject, category).await?;
                Ok(SupportResponse::success(&ticket))
            }
            "add_reply" => {
                let ticket_id = message.payload.get("ticket_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let sender = if message.payload.get("is_agent").and_then(|v| v.as_bool()).unwrap_or(false) {
                    MessageSender::Agent
                } else {
                    MessageSender::Customer
                };
                let sender_name = message.payload.get("sender_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Agent");
                let content = message.payload.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                self.add_reply(ticket_id, sender, sender_name, content).await?;
                Ok(SupportResponse::success(json!({"status": "replied"})))
            }
            "get_customer_tickets" => {
                let customer_id = message.payload.get("customer_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tickets = self.get_customer_tickets(customer_id).await;
                Ok(SupportResponse::success(json!({"tickets": tickets})))
            }
            "create_customer" => {
                let name = message.payload.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Customer");
                let phone = message.payload.get("phone")
                    .and_then(|v| v.as_str());
                let zalo_id = message.payload.get("zalo_id")
                    .and_then(|v| v.as_str());
                
                let mut customer = Customer::new(name);
                if let Some(p) = phone {
                    customer = customer.with_phone(p);
                }
                if let Some(z) = zalo_id {
                    customer = customer.with_zalo(z);
                }
                
                let created = self.create_customer(customer).await?;
                Ok(SupportResponse::success(&created)))
            }
            "find_customer" => {
                let phone = message.payload.get("phone").and_then(|v| v.as_str());
                let zalo = message.payload.get("zalo_id").and_then(|v| v.as_str());
                let customer = self.find_customer(phone, zalo).await;
                Ok(SupportResponse::success(json!({"customer": customer})))
            }
            _ => Ok(SupportResponse::error("Unknown intent"))
        }
    }
}

impl Default for SupportAgent {
    fn default() -> Self {
        Self::new()
    }
}
