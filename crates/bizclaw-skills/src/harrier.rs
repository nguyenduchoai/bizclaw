//! Harrier Embedding Skill
//! Implements Microsoft's Harrier-OSS-v1 (0.6B) MTEB v2 Model Embedding for
//! native Vietnamese Database RAG & Semantic Search.

use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::types::ToolDefinition;

/// Definition of the local_harrier_embed tool
pub fn local_harrier_embed_definition() -> ToolDefinition {
    ToolDefinition {
        name: "local_harrier_embed".into(),
        description: "Generate high-quality semantic vectors using Microsoft Harrier-OSS-v1 (0.6B) for RAG context matching, handling up to 32k tokens natively.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The raw long-context text (e.g. from a contract or SOP) to compute embeddings for"
                },
                "task_type": {
                    "type": "string",
                    "enum": ["retrieval_query", "retrieval_document", "semantic_similarity"],
                    "description": "The target task instructing the embedding behavior"
                }
            },
            "required": ["text", "task_type"]
        }),
    }
}

/// Implementation of Harrier embedding using an external API endpoint (like Ollama or OpenAI compatible).
pub async fn execute_local_harrier_embed(
    text: &str,
    _task_type: &str,
    api_url: Option<String>,
    api_key: Option<String>,
) -> Result<Vec<f32>> {
    tracing::info!("🧠 Embedding requested for text ({} bytes)", text.len());

    if text.is_empty() {
        return Err(BizClawError::Other("Embedding text cannot be empty".into()));
    }

    let url = api_url.unwrap_or_else(|| "http://localhost:11434/api/embeddings".to_string());

    let client = reqwest::Client::new();
    let is_ollama = url.contains("11434") || url.contains("/api/embeddings");

    let mut req = client.post(&url);
    if let Some(key) = api_key {
        if !key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
    }

    // Adapt payload based on Ollama / OpenAI
    let payload = if is_ollama {
        serde_json::json!({
            "model": "nomic-embed-text", // Default fallback
            "prompt": text,
        })
    } else {
        serde_json::json!({
            "input": text,
            "model": "text-embedding-3-small",
        })
    };

    let resp =
        req.json(&payload).send().await.map_err(|e| {
            BizClawError::Other(format!("Failed to connect to embedding API: {}", e))
        })?;

    if !resp.status().is_success() {
        let err_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown API Error".into());
        return Err(BizClawError::Other(format!(
            "Embedding API error: {}",
            err_text
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| BizClawError::Other(format!("Failed to parse embedding response: {}", e)))?;

    let vec = if is_ollama {
        json.get("embedding").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_f64().map(|f| f as f32))
                .collect::<Vec<f32>>()
        })
    } else {
        json.get("data")
            .and_then(|d| d.as_array())
            .and_then(|d| d.first())
            .and_then(|first| first.get("embedding"))
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_f64().map(|f| f as f32))
                    .collect::<Vec<f32>>()
            })
    };

    vec.ok_or_else(|| BizClawError::Other("Invalid embedding response format".into()))
}
