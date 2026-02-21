//! Identity configuration trait.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub persona: String,
    pub system_prompt: String,
}

impl Default for Identity {
    fn default() -> Self {
        Self {
            name: "BizClaw".into(),
            persona: "A helpful AI assistant".into(),
            system_prompt: "You are BizClaw, a fast and capable AI assistant. Be concise and helpful.".into(),
        }
    }
}
