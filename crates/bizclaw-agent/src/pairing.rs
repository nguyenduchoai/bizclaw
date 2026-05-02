//! DM Pairing Codes (8-char TTL).
//! Security layer for channel interactions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingCode {
    pub code: String,
    pub user_id: String,
    pub channel: String,
    pub expires_at: i64,
    pub used: bool,
}

#[derive(Default)]
pub struct PairingManager {
    codes: HashMap<String, PairingCode>,
    ttl_minutes: i64,
}

impl PairingManager {
    pub fn new(ttl_minutes: i64) -> Self {
        Self {
            codes: HashMap::new(),
            ttl_minutes,
        }
    }
    
    pub fn generate_code(&mut self, user_id: &str, channel: &str) -> String {
        let code = Self::random_code(8);
        let expires = chrono::Utc::now().timestamp() + self.ttl_minutes * 60;
        
        self.codes.insert(code.clone(), PairingCode {
            code: code.clone(),
            user_id: user_id.to_string(),
            channel: channel.to_string(),
            expires_at: expires,
            used: false,
        });
        
        code
    }
    
    pub fn verify_code(&self, code: &str) -> Option<PairingCode> {
        let pairing = self.codes.get(code)?;
        
        if pairing.used || pairing.expires_at < chrono::Utc::now().timestamp() {
            return None;
        }
        
        Some(pairing.clone())
    }
    
    pub fn consume_code(&mut self, code: &str) -> Option<PairingCode> {
        let pairing = self.codes.get_mut(code)?;
        
        if pairing.expires_at < chrono::Utc::now().timestamp() {
            return None;
        }
        
        pairing.used = true;
        Some(pairing.clone())
    }
    
    fn random_code(len: usize) -> String {
        const CHARSET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        
        let mut code = String::new();
        let mut hasher = DefaultHasher::new();
        
        for i in 0..len {
            let seed = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64 ^ (i as u64 * 17);
            hasher.write_u64(seed);
            let idx = (hasher.finish() % CHARSET.len() as u64) as usize;
            code.push(CHARSET.chars().nth(idx).unwrap_or('A'));
        }
        
        code
    }
}
