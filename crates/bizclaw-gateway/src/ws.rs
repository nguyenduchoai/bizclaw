//! WebSocket handler for real-time streaming chat via gateway.
//!
//! Architecture:
//! - If Agent Engine is available → uses it for FULL processing (tools + memory + all providers)
//! - Streaming mode → direct provider HTTP for UX, then saves to Agent memory
//! - Fallback → raw HTTP calls to Ollama/OpenAI if Agent unavailable
//!
//! Protocol:
//! → Client sends: {"type":"chat","content":"...","stream":true}
//! ← Server sends: {"type":"chat_start","request_id":"..."}
//! ← Server sends: {"type":"chat_chunk","request_id":"...","content":"token","index":0}
//! ← Server sends: {"type":"chat_done","request_id":"...","total_tokens":42}

use super::server::AppState;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use std::sync::Arc;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Resolve Ollama URL from config or env.
fn ollama_url(_state: &AppState) -> String {
    if let Ok(url) = std::env::var("OLLAMA_HOST") {
        return url;
    }
    "http://localhost:11434".to_string()
}

/// Get the active model from config.
fn active_model(state: &AppState) -> String {
    let config = state.full_config.lock().unwrap();
    let model = config.default_model.clone();
    if model.is_empty() {
        "tinyllama".to_string()
    } else {
        model
    }
}

/// Get the active provider from config.
fn active_provider(state: &AppState) -> String {
    let config = state.full_config.lock().unwrap();
    let provider = config.default_provider.clone();
    if provider.is_empty() {
        "ollama".to_string()
    } else {
        provider
    }
}

/// Handle a WebSocket connection.
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("WebSocket client connected");

    let provider = active_provider(&state);
    let model = active_model(&state);

    // Check if Agent Engine is available at connection start (for welcome msg)
    let has_agent_initial = {
        let agent = state.agent.lock().await;
        agent.is_some()
    };

    // Send welcome with capabilities
    let welcome = serde_json::json!({
        "type": "connected",
        "message": "BizClaw Gateway — WebSocket connected",
        "version": env!("CARGO_PKG_VERSION"),
        "provider": &provider,
        "model": &model,
        "agent_engine": has_agent_initial,
        "capabilities": if has_agent_initial {
            vec!["chat", "stream", "ping", "tools", "memory"]
        } else {
            vec!["chat", "stream", "ping"]
        },
    });
    if send_json(&mut socket, &welcome).await.is_err() {
        return;
    }

    if has_agent_initial {
        tracing::info!("WS session using Agent Engine (tools + memory enabled)");
    } else {
        tracing::info!("WS session using direct provider calls (no tools/memory)");
    }

    let mut request_counter: u64 = 0;
    // Fallback history for direct mode (when Agent engine is not available)
    let mut fallback_history: Vec<serde_json::Value> = vec![
        serde_json::json!({"role": "system", "content": "Bạn là BizClaw AI Assistant. Trả lời ngắn gọn, hữu ích bằng tiếng Việt. Nếu user nói tiếng Anh thì trả lời tiếng Anh."}),
    ];

    // Message loop
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                let json = match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(j) => j,
                    Err(e) => {
                        send_error(&mut socket, &format!("Invalid JSON: {e}")).await;
                        continue;
                    }
                };

                let msg_type = json["type"].as_str().unwrap_or("unknown");

                match msg_type {
                    "chat" => {
                        request_counter += 1;
                        let request_id = format!("req_{request_counter}");
                        let content = json["content"].as_str().unwrap_or("").to_string();
                        let stream = json["stream"].as_bool().unwrap_or(true);

                        if content.is_empty() {
                            send_error(&mut socket, "Empty message").await;
                            continue;
                        }

                        // Re-read provider/model from config each request (may have changed)
                        let provider = active_provider(&state);
                        let model = active_model(&state);

                        // Dynamic agent check: re-check each request so config changes take effect
                        let has_agent = {
                            let agent = state.agent.lock().await;
                            if let Some(ref a) = *agent {
                                // Only use Agent if its provider matches current config provider
                                let agent_provider = a.provider_name().to_string();
                                let providers_match = agent_provider == provider
                                    || (provider == "brain" && agent_provider == "brain")
                                    || (provider == "ollama" && agent_provider == "ollama")
                                    || (provider == "llamacpp" && agent_provider == "llamacpp");
                                if !providers_match {
                                    tracing::warn!(
                                        "Agent provider ({}) != config provider ({}), using direct mode",
                                        agent_provider,
                                        provider
                                    );
                                }
                                providers_match
                            } else {
                                false
                            }
                        };

                        tracing::info!(
                            "Chat req={request_id}: provider={provider}, model={model}, stream={stream}, len={}, agent={has_agent}",
                            content.len()
                        );

                        if has_agent {
                            // ═══════════════════════════════════════════
                            // AGENT ENGINE MODE (tools + memory + all providers)
                            // Works for BOTH stream and non-stream requests
                            // ═══════════════════════════════════════════
                            let _ = send_json(
                                &mut socket,
                                &serde_json::json!({
                                    "type": "chat_start",
                                    "request_id": &request_id,
                                    "provider": &provider,
                                    "model": &model,
                                    "mode": "agent",
                                }),
                            )
                            .await;

                            let result = {
                                let mut agent = state.agent.lock().await;
                                if let Some(agent) = agent.as_mut() {
                                    // Connect knowledge base for RAG
                                    agent.set_knowledge(state.knowledge.clone());
                                    Some(agent.process(&content).await)
                                } else {
                                    None
                                }
                            };

                            // Get context stats after processing
                            let ctx_stats = {
                                let agent = state.agent.lock().await;
                                agent.as_ref().map(|a| a.context_stats().clone())
                            };

                            match result {
                                Some(Ok(response)) => {
                                    if stream {
                                        // Emit as rapid chunks for streaming UX
                                        let chunk_size = 8; // chars per chunk
                                        let chars: Vec<char> = response.chars().collect();
                                        let mut idx: u64 = 0;
                                        for chunk in chars.chunks(chunk_size) {
                                            let text: String = chunk.iter().collect();
                                            let _ = send_json(
                                                &mut socket,
                                                &serde_json::json!({
                                                    "type": "chat_chunk",
                                                    "request_id": &request_id,
                                                    "content": &text,
                                                    "index": idx,
                                                }),
                                            )
                                            .await;
                                            idx += 1;
                                        }
                                        let _ = send_json(
                                            &mut socket,
                                            &serde_json::json!({
                                                "type": "chat_done",
                                                "request_id": &request_id,
                                                "total_tokens": idx,
                                                "full_content": &response,
                                                "mode": "agent",
                                                "context": ctx_stats,
                                            }),
                                        )
                                        .await;
                                    } else {
                                        let _ = send_json(
                                            &mut socket,
                                            &serde_json::json!({
                                                "type": "chat_response",
                                                "request_id": &request_id,
                                                "content": &response,
                                                "provider": &provider,
                                                "model": &model,
                                                "mode": "agent",
                                            }),
                                        )
                                        .await;
                                        let _ = send_json(
                                            &mut socket,
                                            &serde_json::json!({
                                                "type": "chat_done",
                                                "request_id": &request_id,
                                                "full_content": &response,
                                                "mode": "agent",
                                            }),
                                        )
                                        .await;
                                    }
                                }
                                Some(Err(e)) => {
                                    let _ = send_json(
                                        &mut socket,
                                        &serde_json::json!({
                                            "type": "chat_error",
                                            "request_id": &request_id,
                                            "error": e.to_string(),
                                        }),
                                    )
                                    .await;
                                }
                                None => {
                                    send_error(&mut socket, "Agent engine not available").await;
                                }
                            }
                        } else {
                            // ═══════════════════════════════════════════
                            // STREAMING / DIRECT MODE
                            // ═══════════════════════════════════════════
                            // Add user message to fallback history
                            fallback_history
                                .push(serde_json::json!({"role": "user", "content": &content}));

                            // Keep history manageable (last 20 messages + system)
                            if fallback_history.len() > 21 {
                                let system = fallback_history[0].clone();
                                let skip = fallback_history.len() - 20;
                                let tail: Vec<_> = fallback_history.drain(skip..).collect();
                                fallback_history.clear();
                                fallback_history.push(system);
                                fallback_history.extend(tail);
                            }

                            // Route to provider
                            let result = match provider.as_str() {
                                "ollama" | "brain" => {
                                    chat_ollama(
                                        &mut socket,
                                        &state,
                                        &request_id,
                                        &fallback_history,
                                        &model,
                                        stream,
                                    )
                                    .await
                                }
                                "openai" => {
                                    chat_openai(
                                        &mut socket,
                                        &state,
                                        &request_id,
                                        &fallback_history,
                                        &model,
                                        stream,
                                    )
                                    .await
                                }
                                _ => {
                                    // Fallback: try Ollama first, then OpenAI
                                    let r = chat_ollama(
                                        &mut socket,
                                        &state,
                                        &request_id,
                                        &fallback_history,
                                        &model,
                                        stream,
                                    )
                                    .await;
                                    if r.is_err() {
                                        chat_openai(
                                            &mut socket,
                                            &state,
                                            &request_id,
                                            &fallback_history,
                                            "gpt-4o-mini",
                                            stream,
                                        )
                                        .await
                                    } else {
                                        r
                                    }
                                }
                            };

                            match result {
                                Ok(response) => {
                                    // Add assistant response to fallback history
                                    fallback_history.push(serde_json::json!({"role": "assistant", "content": &response}));

                                    // Save to Agent memory if any agent exists (memory is provider-agnostic)
                                    {
                                        let mut agent = state.agent.lock().await;
                                        if let Some(agent) = agent.as_mut() {
                                            agent.save_memory_public(&content, &response).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = send_json(
                                        &mut socket,
                                        &serde_json::json!({
                                            "type": "chat_error",
                                            "request_id": &request_id,
                                            "error": e,
                                        }),
                                    )
                                    .await;
                                }
                            }
                        }
                    }

                    "ping" => {
                        let pong = serde_json::json!({
                            "type": "pong",
                            "timestamp": chrono::Utc::now().timestamp_millis(),
                        });
                        let _ = send_json(&mut socket, &pong).await;
                    }

                    "status" => {
                        let current_provider = active_provider(&state);
                        let current_model = active_model(&state);
                        let agent_info = {
                            let agent = state.agent.lock().await;
                            if let Some(agent) = agent.as_ref() {
                                serde_json::json!({
                                    "provider": agent.provider_name(),
                                    "conversation_length": agent.conversation().len(),
                                    "tools_available": true,
                                    "memory_enabled": true,
                                })
                            } else {
                                serde_json::json!(null)
                            }
                        };

                        let status = serde_json::json!({
                            "type": "status",
                            "requests_processed": request_counter,
                            "uptime_secs": state.start_time.elapsed().as_secs(),
                            "provider": &current_provider,
                            "model": &current_model,
                            "agent_engine": agent_info != serde_json::json!(null),
                            "agent": agent_info,
                        });
                        let _ = send_json(&mut socket, &status).await;
                    }

                    _ => {
                        send_error(&mut socket, &format!("Unknown message type: {msg_type}")).await;
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                let _ = socket.send(Message::Pong(data)).await;
            }
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket client disconnected (close frame)");
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {e}");
                break;
            }
            _ => {}
        }
    }

    tracing::info!("WebSocket connection closed (total requests: {request_counter})");
}

// ═══════════════════════════════════════════════════════════
// OLLAMA PROVIDER
// ═══════════════════════════════════════════════════════════

async fn chat_ollama(
    socket: &mut WebSocket,
    state: &AppState,
    request_id: &str,
    messages: &[serde_json::Value],
    model: &str,
    stream: bool,
) -> Result<String, String> {
    let url = ollama_url(state);
    let client = reqwest::Client::new();

    if stream {
        // Streaming response
        let _ = send_json(
            socket,
            &serde_json::json!({
                "type": "chat_start",
                "request_id": request_id,
                "provider": "ollama",
                "model": model,
            }),
        )
        .await;

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        let resp = client
            .post(format!("{url}/api/chat"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama connection failed ({}): {}", url, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama error {status}: {text}"));
        }

        let mut full_content = String::new();
        let mut chunk_idx: u64 = 0;

        // Read streaming NDJSON response
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&bytes);

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(content) = json["message"]["content"].as_str() {
                    if !content.is_empty() {
                        full_content.push_str(content);
                        let _ = send_json(
                            socket,
                            &serde_json::json!({
                                "type": "chat_chunk",
                                "request_id": request_id,
                                "content": content,
                                "index": chunk_idx,
                            }),
                        )
                        .await;
                        chunk_idx += 1;
                    }
                }
            }
        }

        let _ = send_json(
            socket,
            &serde_json::json!({
                "type": "chat_done",
                "request_id": request_id,
                "total_tokens": chunk_idx,
                "full_content": &full_content,
            }),
        )
        .await;

        Ok(full_content)
    } else {
        // Non-streaming
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let resp = client
            .post(format!("{url}/api/chat"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama connection failed: {e}"))?;

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let _ = send_json(
            socket,
            &serde_json::json!({
                "type": "chat_response",
                "request_id": request_id,
                "content": &content,
                "provider": "ollama",
                "model": model,
            }),
        )
        .await;

        Ok(content)
    }
}

// ═══════════════════════════════════════════════════════════
// OPENAI PROVIDER
// ═══════════════════════════════════════════════════════════

async fn chat_openai(
    socket: &mut WebSocket,
    state: &AppState,
    request_id: &str,
    messages: &[serde_json::Value],
    model: &str,
    stream: bool,
) -> Result<String, String> {
    let api_key = {
        let config = state.full_config.lock().unwrap();
        config.api_key.clone()
    };
    let api_key = if api_key.is_empty() {
        std::env::var("OPENAI_API_KEY").map_err(|_| {
            "OpenAI API key not configured. Set in Settings → API Key or OPENAI_API_KEY env var"
                .to_string()
        })?
    } else {
        api_key
    };

    let client = reqwest::Client::new();

    if stream {
        // Streaming SSE mode
        let _ = send_json(
            socket,
            &serde_json::json!({
                "type": "chat_start",
                "request_id": request_id,
                "provider": "openai",
                "model": model,
            }),
        )
        .await;

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        let resp = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI error: {text}"));
        }

        // Read SSE stream
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&bytes);
        let mut full_content = String::new();
        let mut chunk_idx: u64 = 0;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                        if !content.is_empty() {
                            full_content.push_str(content);
                            let _ = send_json(
                                socket,
                                &serde_json::json!({
                                    "type": "chat_chunk",
                                    "request_id": request_id,
                                    "content": content,
                                    "index": chunk_idx,
                                }),
                            )
                            .await;
                            chunk_idx += 1;
                        }
                    }
                }
            }
        }

        let _ = send_json(
            socket,
            &serde_json::json!({
                "type": "chat_done",
                "request_id": request_id,
                "total_tokens": chunk_idx,
                "full_content": &full_content,
            }),
        )
        .await;

        Ok(full_content)
    } else {
        // Non-streaming mode
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
        });

        let resp = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI error: {text}"));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let _ = send_json(
            socket,
            &serde_json::json!({
                "type": "chat_response",
                "request_id": request_id,
                "content": &content,
                "provider": "openai",
                "model": model,
            }),
        )
        .await;

        Ok(content)
    }
}

// ═══════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════

async fn send_json(socket: &mut WebSocket, value: &serde_json::Value) -> Result<(), ()> {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .map_err(|e| {
            tracing::error!("WS send failed: {e}");
        })
}

async fn send_error(socket: &mut WebSocket, message: &str) {
    let error = serde_json::json!({
        "type": "error",
        "message": message,
    });
    let _ = send_json(socket, &error).await;
}
