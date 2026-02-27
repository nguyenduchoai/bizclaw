//! OpenAI-Compatible API — drop-in replacement for `/v1/chat/completions` and `/v1/models`.
//!
//! Any tool/app that supports OpenAI API (Cursor, Continue, Aider, LibreChat, etc.)
//! can use BizClaw as a proxy by pointing to `http://localhost:3579/v1`.
//!
//! Authentication: `Authorization: Bearer <pairing-code>` or `api-key` header.

use axum::extract::State;
use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use super::server::AppState;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: UsageResponse,
}

#[derive(Debug, Serialize)]
pub struct Choice {
    pub index: usize,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct ModelObject {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

#[derive(Debug, Serialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<ModelObject>,
}

// ─── Auth helper ─────────────────────────────────────────────────────────────

/// Extract API key from Authorization header (Bearer token) or x-api-key header.
fn extract_api_key(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try Authorization: Bearer <key>
    if let Some(auth) = headers.get("authorization")
        && let Ok(val) = auth.to_str()
            && let Some(token) = val.strip_prefix("Bearer ") {
                return Some(token.trim().to_string());
            }
    // Try x-api-key header
    if let Some(key) = headers.get("x-api-key")
        && let Ok(val) = key.to_str() {
            return Some(val.trim().to_string());
        }
    None
}

/// Validate API key against pairing code. Returns true if valid.
fn validate_key(state: &AppState, key: &str) -> bool {
    let stored = state.pairing_code.lock().unwrap().clone();
    // Constant-time comparison
    if key.len() != stored.len() {
        return false;
    }
    key.as_bytes()
        .iter()
        .zip(stored.as_bytes().iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

// ─── POST /v1/chat/completions ───────────────────────────────────────────────

pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Json<Value>, StatusCode> {
    // Auth check
    let key = extract_api_key(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    if !validate_key(&state, &key) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let start = std::time::Instant::now();

    // Route "model" field to agent name — if model matches an agent, use it
    // Otherwise use the default agent
    let user_content = req.messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .and_then(|m| m.content.as_deref())
        .unwrap_or("");

    let response_text = {
        // Try to find agent by model name first
        let mut orch = state.orchestrator.lock().await;
        if let Some(agent) = orch.get_agent_mut(&req.model) {
            // Use the named agent
            match agent.process(user_content).await {
                Ok(r) => r,
                Err(e) => format!("Error: {e}"),
            }
        } else {
            // Fallback to default agent
            drop(orch);
            let mut agent_lock = state.agent.lock().await;
            if let Some(agent) = agent_lock.as_mut() {
                match agent.process(user_content).await {
                    Ok(r) => r,
                    Err(e) => format!("Error: {e}"),
                }
            } else {
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        }
    };

    let elapsed = start.elapsed();
    let est_prompt_tokens = (user_content.len() / 4) as u32;
    let est_completion_tokens = (response_text.len() / 4) as u32;

    // Record trace
    {
        let trace = LlmTrace {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            model: req.model.clone(),
            provider: "bizclaw".into(),
            prompt_tokens: est_prompt_tokens,
            completion_tokens: est_completion_tokens,
            total_tokens: est_prompt_tokens + est_completion_tokens,
            latency_ms: elapsed.as_millis() as u64,
            cost_usd: estimate_cost(&req.model, est_prompt_tokens, est_completion_tokens),
            cache_hit: false,
            status: "ok".into(),
            tool_calls: 0,
            error: None,
        };
        let mut traces = state.traces.lock().unwrap();
        // Cap at 10,000 traces to prevent unbounded memory growth
        if traces.len() >= 10_000 {
            traces.drain(..1_000); // Remove oldest 1,000 when full
        }
        traces.push(trace);
    }

    // Broadcast activity event via WebSocket
    let _ = state.activity_tx.send(ActivityEvent {
        event_type: "llm.completed".into(),
        agent: req.model.clone(),
        detail: format!("{}tok in {}ms", est_prompt_tokens + est_completion_tokens, elapsed.as_millis()),
        timestamp: chrono::Utc::now(),
    });

    let response = json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..24].to_string()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": req.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": response_text,
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": est_prompt_tokens,
            "completion_tokens": est_completion_tokens,
            "total_tokens": est_prompt_tokens + est_completion_tokens,
        }
    });

    Ok(Json(response))
}

// ─── GET /v1/models ──────────────────────────────────────────────────────────

pub async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    // Auth check
    let key = extract_api_key(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    if !validate_key(&state, &key) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // List all agents as "models"
    let orch = state.orchestrator.lock().await;
    let agents = orch.list_agents();

    let mut models: Vec<Value> = agents.iter().map(|a| {
        json!({
            "id": a["name"].as_str().unwrap_or("default"),
            "object": "model",
            "created": chrono::Utc::now().timestamp(),
            "owned_by": format!("bizclaw:{}", a["provider"].as_str().unwrap_or("unknown")),
        })
    }).collect();

    // Also add "default" model
    models.push(json!({
        "id": "default",
        "object": "model",
        "created": chrono::Utc::now().timestamp(),
        "owned_by": "bizclaw",
    }));

    Ok(Json(json!({
        "object": "list",
        "data": models,
    })))
}

// ─── Tracing Types ───────────────────────────────────────────────────────────

/// LLM call trace — records every provider call for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTrace {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub latency_ms: u64,
    pub cost_usd: f64,
    pub cache_hit: bool,
    pub status: String,
    pub tool_calls: u32,
    pub error: Option<String>,
}

/// Real-time activity event — broadcast to all connected dashboards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub event_type: String,
    pub agent: String,
    pub detail: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ─── Cost estimation ─────────────────────────────────────────────────────────

/// Rough cost estimation per model (USD per 1M tokens).
fn estimate_cost(model: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
    let (input_rate, output_rate) = match model {
        m if m.contains("gpt-4o-mini") => (0.15, 0.60),
        m if m.contains("gpt-4o") => (2.50, 10.00),
        m if m.contains("gpt-4") => (30.00, 60.00),
        m if m.contains("claude-3-5-sonnet") || m.contains("claude-4") => (3.00, 15.00),
        m if m.contains("claude-3-5-haiku") => (0.80, 4.00),
        m if m.contains("gemini-2.0-flash") => (0.075, 0.30),
        m if m.contains("gemini") => (0.50, 1.50),
        m if m.contains("deepseek") => (0.14, 0.28),
        m if m.contains("groq") || m.contains("llama") => (0.05, 0.10),
        m if m.contains("mistral") => (0.25, 0.25),
        _ => (1.00, 3.00), // default estimate
    };

    let input_cost = (prompt_tokens as f64 / 1_000_000.0) * input_rate;
    let output_cost = (completion_tokens as f64 / 1_000_000.0) * output_rate;
    ((input_cost + output_cost) * 100_000.0).round() / 100_000.0
}

// ─── Trace API Handlers ──────────────────────────────────────────────────────

/// GET /api/v1/traces — list recent LLM call traces.
pub async fn list_traces(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let traces = state.traces.lock().unwrap();
    let recent: Vec<_> = traces.iter().rev().take(100).cloned().collect();

    // Aggregate stats
    let total_cost: f64 = traces.iter().map(|t| t.cost_usd).sum();
    let total_tokens: u64 = traces.iter().map(|t| t.total_tokens as u64).sum();
    let avg_latency: f64 = if traces.is_empty() {
        0.0
    } else {
        traces.iter().map(|t| t.latency_ms as f64).sum::<f64>() / traces.len() as f64
    };
    let cache_hits = traces.iter().filter(|t| t.cache_hit).count();

    Json(json!({
        "ok": true,
        "traces": recent,
        "stats": {
            "total_calls": traces.len(),
            "total_cost_usd": total_cost,
            "total_tokens": total_tokens,
            "avg_latency_ms": avg_latency.round(),
            "cache_hit_rate": if traces.is_empty() { 0.0 } else { cache_hits as f64 / traces.len() as f64 },
        }
    }))
}

/// GET /api/v1/traces/cost — cost breakdown by model/provider.
pub async fn cost_breakdown(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let traces = state.traces.lock().unwrap();

    let mut by_model: std::collections::HashMap<String, (f64, u64, usize)> = std::collections::HashMap::new();
    for t in traces.iter() {
        let entry = by_model.entry(t.model.clone()).or_insert((0.0, 0, 0));
        entry.0 += t.cost_usd;
        entry.1 += t.total_tokens as u64;
        entry.2 += 1;
    }

    let breakdown: Vec<_> = by_model.iter().map(|(model, (cost, tokens, calls))| {
        json!({
            "model": model,
            "cost_usd": cost,
            "total_tokens": tokens,
            "calls": calls,
        })
    }).collect();

    let total: f64 = by_model.values().map(|(c, _, _)| c).sum();

    Json(json!({
        "ok": true,
        "breakdown": breakdown,
        "total_cost_usd": total,
        "period": "session", // Since last restart
    }))
}

/// GET /api/v1/activity — recent activity events.
pub async fn list_activity(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let events = state.activity_log.lock().unwrap();
    let recent: Vec<_> = events.iter().rev().take(50).cloned().collect();
    Json(json!({
        "ok": true,
        "events": recent,
        "total": events.len(),
    }))
}
