use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub zalo_id: Option<String>,
    pub segment: CustomerSegment,
    pub total_spent: i64,
    pub order_count: u32,
    pub ticket_count: u32,
    pub last_order_at: Option<DateTime<Utc>>,
    pub last_ticket_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub notes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CustomerSegment {
    VIP,
    Regular,
    New,
    AtRisk,
    Churned,
}

impl Customer {
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            phone: None,
            email: None,
            zalo_id: None,
            segment: CustomerSegment::New,
            total_spent: 0,
            order_count: 0,
            ticket_count: 0,
            last_order_at: None,
            last_ticket_at: None,
            tags: vec![],
            notes: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_phone(mut self, phone: &str) -> Self {
        self.phone = Some(phone.to_string());
        self
    }

    pub fn with_zalo(mut self, zalo_id: &str) -> Self {
        self.zalo_id = Some(zalo_id.to_string());
        self
    }

    pub fn add_order(&mut self, amount: i64) {
        self.total_spent += amount;
        self.order_count += 1;
        self.last_order_at = Some(Utc::now());
        self.update_segment();
        self.updated_at = Utc::now();
    }

    pub fn add_ticket(&mut self) {
        self.ticket_count += 1;
        self.last_ticket_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    fn update_segment(&mut self) {
        if self.order_count == 0 {
            self.segment = CustomerSegment::New;
        } else if self.total_spent > 10_000_000 {
            self.segment = CustomerSegment::VIP;
        } else if self.total_spent > 1_000_000 {
            self.segment = CustomerSegment::Regular;
        } else {
            self.segment = CustomerSegment::New;
        }
    }

    pub fn average_order_value(&self) -> i64 {
        if self.order_count > 0 {
            self.total_spent / self.order_count as i64
        } else {
            0
        }
    }
}

pub struct CustomerManager {
    customers: std::collections::HashMap<String, Customer>,
    by_phone: std::collections::HashMap<String, String>,
    by_zalo: std::collections::HashMap<String, String>,
}

impl CustomerManager {
    pub fn new() -> Self {
        Self {
            customers: std::collections::HashMap::new(),
            by_phone: std::collections::HashMap::new(),
            by_zalo: std::collections::HashMap::new(),
        }
    }

    pub async fn create_customer(&mut self, customer: Customer) -> anyhow::Result<Customer> {
        let id = customer.id.clone();
        self.customers.insert(id.clone(), customer.clone());
        
        if let Some(ref phone) = customer.phone {
            self.by_phone.insert(phone.clone(), id.clone());
        }
        if let Some(ref zalo) = customer.zalo_id {
            self.by_zalo.insert(zalo.clone(), id.clone());
        }
        
        Ok(customer)
    }

    pub async fn find_by_phone(&self, phone: &str) -> Option<&Customer> {
        self.by_phone.get(phone).and_then(|id| self.customers.get(id))
    }

    pub async fn find_by_zalo(&self, zalo_id: &str) -> Option<&Customer> {
        self.by_zalo.get(zalo_id).and_then(|id| self.customers.get(id))
    }

    pub async fn get_vip_customers(&self) -> Vec<&Customer> {
        self.customers.values()
            .filter(|c| c.segment == CustomerSegment::VIP)
            .collect()
    }
}

impl Default for CustomerManager {
    fn default() -> Self {
        Self::new()
    }
}
