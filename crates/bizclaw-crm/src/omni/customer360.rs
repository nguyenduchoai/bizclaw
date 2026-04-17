//! Customer 360 Manager

use crate::omni::{
    ChannelAccount, ChannelContact, DedupeEngine, Interaction, Review, SupportTicket, Transaction,
    UnifiedContact,
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Customer360Manager {
    contacts: Arc<RwLock<HashMap<String, UnifiedContact>>>,
    interactions: Arc<RwLock<HashMap<String, Vec<Interaction>>>>,
    transactions: Arc<RwLock<HashMap<String, Vec<Transaction>>>>,
    tickets: Arc<RwLock<HashMap<String, Vec<SupportTicket>>>>,
    reviews: Arc<RwLock<HashMap<String, Vec<Review>>>>,
    accounts: Arc<RwLock<HashMap<String, ChannelAccount>>>,
    deduplication: DedupeEngine,
}

impl Customer360Manager {
    pub fn new() -> Self {
        Self {
            contacts: Arc::new(RwLock::new(HashMap::new())),
            interactions: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            tickets: Arc::new(RwLock::new(HashMap::new())),
            reviews: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            deduplication: DedupeEngine::new(),
        }
    }

    pub async fn register_account(&self, account: ChannelAccount) -> Result<()> {
        self.accounts
            .write()
            .await
            .insert(account.id.clone(), account);
        Ok(())
    }

    pub async fn upsert_contact(&self, contact: ChannelContact) -> Result<String> {
        let unified = self.deduplication.merge_contacts(vec![contact]);
        self.contacts
            .write()
            .await
            .insert(unified.id.clone(), unified.clone());
        Ok(unified.id)
    }

    pub async fn add_interaction(&self, contact_id: &str, interaction: Interaction) -> Result<()> {
        let mut map = self.interactions.write().await;
        map.entry(contact_id.to_string())
            .or_insert_with(Vec::new)
            .push(interaction);
        Ok(())
    }

    pub async fn add_transaction(&self, contact_id: &str, transaction: Transaction) -> Result<()> {
        let mut map = self.transactions.write().await;
        map.entry(contact_id.to_string())
            .or_insert_with(Vec::new)
            .push(transaction);
        Ok(())
    }

    pub async fn add_ticket(&self, contact_id: &str, ticket: SupportTicket) -> Result<()> {
        let mut map = self.tickets.write().await;
        map.entry(contact_id.to_string())
            .or_insert_with(Vec::new)
            .push(ticket);
        Ok(())
    }

    pub async fn add_review(&self, contact_id: &str, review: Review) -> Result<()> {
        let mut map = self.reviews.write().await;
        map.entry(contact_id.to_string())
            .or_insert_with(Vec::new)
            .push(review);
        Ok(())
    }

    pub async fn search(&self, query: &str) -> Vec<UnifiedContact> {
        let map = self.contacts.read().await;
        let query_lower = query.to_lowercase();
        map.values()
            .filter(|c| c.primary_name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }
}

impl Default for Customer360Manager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Customer360 {
    pub contact: UnifiedContact,
    pub interactions: Vec<Interaction>,
    pub transactions: Vec<Transaction>,
    pub tickets: Vec<SupportTicket>,
    pub reviews: Vec<Review>,
}
