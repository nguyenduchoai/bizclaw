use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub id: String,
    pub number: String,
    pub contract_type: ContractType,
    pub status: ContractStatus,
    pub title: String,
    pub parties: Vec<Party>,
    pub content: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub value: Option<i64>,
    pub currency: String,
    pub terms: Vec<ContractTerm>,
    pub attachments: Vec<String>,
    pub signatures: Vec<Signature>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: String,
    pub name: String,
    pub role: PartyRole,
    pub address: Option<String>,
    pub representative: Option<String>,
    pub tax_code: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyRole {
    Customer,
    Vendor,
    Partner,
    Employee,
    Contractor,
    Investor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContractType {
    Service,
    Sales,
    Purchase,
    Employment,
    Consulting,
    Lease,
    NDA,
    MOU,
    Distribution,
    Franchise,
    JointVenture,
    Supply,
    Commission,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    Draft,
    Review,
    PendingSignature,
    Active,
    Expired,
    Terminated,
    Renewed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTerm {
    pub id: String,
    pub clause: String,
    pub content: String,
    pub category: TermCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermCategory {
    Payment,
    Delivery,
    Warranty,
    Confidentiality,
    Termination,
    Liability,
    Dispute,
    IntellectualProperty,
    ForceMajeure,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub party_id: String,
    pub signed_at: DateTime<Utc>,
    pub signature_data: String,
    pub ip_address: Option<String>,
}

impl Contract {
    pub fn new(contract_type: ContractType, title: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            number: Self::generate_number(),
            contract_type,
            status: ContractStatus::Draft,
            title: title.to_string(),
            parties: vec![],
            content: String::new(),
            start_date: now,
            end_date: None,
            value: None,
            currency: "VND".to_string(),
            terms: vec![],
            attachments: vec![],
            signatures: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    fn generate_number() -> String {
        let now = Utc::now();
        let sequence = rand::random::<u32>() % 10000;
        format!("HD-{}{:02}{:02}-{:04}", now.format("%Y"), now.format("%m"), now.format("%d"), sequence)
    }

    pub fn add_party(&mut self, party: Party) {
        self.parties.push(party);
    }

    pub fn add_term(&mut self, clause: &str, content: &str, category: TermCategory) {
        self.terms.push(ContractTerm {
            id: Uuid::new_v4().to_string(),
            clause: clause.to_string(),
            content: content.to_string(),
            category,
        });
    }

    pub fn sign(&mut self, party_id: &str, signature_data: &str) {
        self.signatures.push(Signature {
            party_id: party_id.to_string(),
            signed_at: Utc::now(),
            signature_data: signature_data.to_string(),
            ip_address: None,
        });
        
        if self.signatures.len() == self.parties.len() {
            self.status = ContractStatus::Active;
        } else {
            self.status = ContractStatus::PendingSignature;
        }
        self.updated_at = Utc::now();
    }

    pub fn is_expired(&self) -> bool {
        if let Some(end_date) = self.end_date {
            Utc::now() > end_date
        } else {
            false
        }
    }
}

pub struct ContractManager {
    contracts: std::collections::HashMap<String, Contract>,
}

impl ContractManager {
    pub fn new() -> Self {
        Self {
            contracts: std::collections::HashMap::new(),
        }
    }

    pub async fn create_contract(&mut self, contract: Contract) -> anyhow::Result<Contract> {
        self.contracts.insert(contract.id.clone(), contract.clone());
        Ok(contract)
    }

    pub async fn get_contract(&self, id: &str) -> anyhow::Result<Option<&Contract>> {
        Ok(self.contracts.get(id))
    }

    pub async fn update_contract(&mut self, contract: Contract) -> anyhow::Result<()> {
        self.contracts.insert(contract.id.clone(), contract);
        Ok(())
    }

    pub async fn list_contracts(&self, status: Option<ContractStatus>) -> Vec<&Contract> {
        match status {
            Some(status) => self.contracts.values().filter(|c| c.status == status).collect(),
            None => self.contracts.values().collect(),
        }
    }
}

impl Default for ContractManager {
    fn default() -> Self {
        Self::new()
    }
}
