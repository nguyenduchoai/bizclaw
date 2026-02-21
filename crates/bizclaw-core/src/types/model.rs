//! Model information types.

use serde::{Deserialize, Serialize};

/// Information about an available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: u32,
    pub max_output_tokens: Option<u32>,
}
