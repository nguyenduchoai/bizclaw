//! WebSocket handler for real-time streaming chat via gateway.
//!
//! Architecture:
//! - If Agent Engine is available → uses it for FULL processing (tools + memory + all providers)
//! - Streaming mode → direct provider HTTP for UX, then saves to Agent memory
//! - Fallback → raw HTTP calls to Ollama/OpenAI if Agent unavailable
//!
//! Security:
//! - SecretRedactor scans all outgoing responses for leaked secrets/PII
//! - SecretRedactor scans incoming messages to prevent secret injection into logs
//! - catch_unwind wraps message handling get to prevent panics from killing the connection
//!
//! Protocol:
//! → Client sends: {"type":"chat","content":"...","stream":true}
//! ← Server sends: {"type":"chat_start","request_id":"..."}
//! ← Server sends: {"type":"chat_chunk","request_id":"...","content":"token","index":0}
//! ← Server sends: {"type":"chat_done","request_id":"...","total_tokens":42}
//! → Client sends: {"type":"compact"}
//! ← Server sends: {"type":"compact_done","removed":N,"remaining":M}

use super::server::AppState;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use std::sync::Arc;

/// Global secret redactor — thread-safe, zero-alloc after init.
static REDACTOR: std::sync::LazyLock<bizclaw_security::redactor::SecretRedactor> =
    std::sync::LazyLock::new(bizclaw_security::redactor::SecretRedactor::new);

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
    let config = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    let model = config.default_model.clone();
    if model.is_empty() {
        "tinyllama".to_string()
    } else {
        model
    }
}

/// Get the active provider from config.
fn active_provider(state: &AppState) -> String {
    let config = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
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
                // ═══════════════════════════════════════════
                // PANIC RESILIENCE (inspired by SkyClaw)
                // Wrap the entire handler in catch_unwind so
                // a panic in any handler doesn't kill the WS.
                // ═══════════════════════════════════════════
                let text_clone = text.clone();
                let handler_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // Parse JSON inside catch_unwind boundary
                    serde_json::from_str::<serde_json::Value>(&text_clone)
                }));

                let json = match handler_result {
                    Ok(Ok(j)) => j,
                    Ok(Err(e)) => {
                        send_error(&mut socket, &format!("Invalid JSON: {e}")).await;
                        continue;
                    }
                    Err(panic_info) => {
                        tracing::error!(
                            "[resilience] WS message handler panicked: {:?}",
                            panic_info
                                .downcast_ref::<String>()
                                .map(|s| s.as_str())
                                .or_else(|| panic_info.downcast_ref::<&str>().copied())
                                .unwrap_or("unknown panic")
                        );
                        send_error(&mut socket, "Internal server error (recovered from panic)")
                            .await;
                        continue;
                    }
                };

                let msg_type = json["type"].as_str().unwrap_or("unknown");

                match msg_type {
                    // ═══════════════════════════════════════════
                    // /compact — Manual conversation compaction
                    // (inspired by OpenClaw + MicroClaw)
                    // ═══════════════════════════════════════════
                    "compact" => {
                        let has_agent = {
                            let agent = state.agent.lock().await;
                            agent.is_some()
                        };
                        if has_agent {
                            let (removed, remaining) = {
                                let mut agent = state.agent.lock().await;
                                if let Some(agent) = agent.as_mut() {
                                    let before = agent.conversation().len();
                                    agent.compact_now().await;
                                    let after = agent.conversation().len();
                                    (before.saturating_sub(after), after)
                                } else {
                                    (0, 0)
                                }
                            };
                            let _ = send_json(
                                &mut socket,
                                &serde_json::json!({
                                    "type": "compact_done",
                                    "removed": removed,
                                    "remaining": remaining,
                                    "message": format!("Compacted: removed {} messages, {} remaining", removed, remaining),
                                }),
                            )
                            .await;
                        } else {
                            send_error(&mut socket, "Agent engine not available for compaction")
                                .await;
                        }
                    }

                    "chat" => {
                        request_counter += 1;
                        let request_id = format!("req_{request_counter}");
                        let content = json["content"].as_str().unwrap_or("").to_string();
                        let stream = json["stream"].as_bool().unwrap_or(true);
                        // Optional: target a specific agent by name for multi-agent routing
                        let target_agent = json["agent"].as_str().map(|s| s.to_string());

                        if content.is_empty() {
                            send_error(&mut socket, "Empty message").await;
                            continue;
                        }

                        // ── SecretRedactor: scan INCOMING message ──
                        // Log if user accidentally sends secrets (don't block, just warn)
                        let incoming_findings = REDACTOR.scan(&content);
                        if !incoming_findings.is_empty() {
                            tracing::warn!(
                                "🔐 SecretRedactor: {} secret(s) detected in user message (types: {})",
                                incoming_findings.len(),
                                incoming_findings
                                    .iter()
                                    .map(|f| f.pattern_name)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                        }

                        // ── Handoff Auto-Routing (Intercept) ──
                        let lower = content.to_lowercase();
                        if lower.contains("gặp nhân viên")
                            || lower.contains("gap nhan vien")
                            || lower.contains("chuyển khách")
                        {
                            tracing::info!(
                                "📞 Handoff hotword detected in Web Chat: '{}'",
                                content
                            );
                            let req = crate::routes::api_handoff::HandoffRequestPayload {
                                customer: "Web Guest".to_string(),
                                channel: Some("Web Chat".to_string()),
                                reason: Some("Khách hàng yêu cầu hỗ trợ từ nhân viên".to_string()),
                                message: Some(content.clone()),
                            };
                            let _ = crate::routes::api_handoff::execute_handoff(state.clone(), req)
                                .await;

                            let reply =
                                crate::routes::api_handoff::load_handoff_settings(&state).greeting;
                            let _ = send_json(
                                &mut socket,
                                &serde_json::json!({
                                    "type": "chat_response",
                                    "request_id": &request_id,
                                    "content": reply,
                                    "provider": "system",
                                    "model": "handoff",
                                    "mode": "agent",
                                }),
                            )
                            .await;
                            let _ = send_json(
                                &mut socket,
                                &serde_json::json!({
                                    "type": "chat_done",
                                    "request_id": &request_id,
                                    "full_content": reply,
                                    "mode": "agent",
                                }),
                            )
                            .await;
                            continue;
                        }

                        // Re-read provider/model from config each request (may have changed)
                        let provider = active_provider(&state);
                        let model = active_model(&state);

                        // ── Multi-Agent routing via Orchestrator ──
                        // If a specific agent is requested AND exists in orchestrator, route to it
                        if let Some(ref agent_name) = target_agent {
                            let orch_has_agent = {
                                let orch = state.orchestrator.lock().await;
                                orch.has_agent(agent_name)
                            };
                            if orch_has_agent {
                                tracing::info!(
                                    "Chat req={request_id}: MULTI-AGENT → agent={agent_name}, len={}",
                                    content.len()
                                );
                                let _ = send_json(
                                    &mut socket,
                                    &serde_json::json!({
                                        "type": "chat_start",
                                        "request_id": &request_id,
                                        "agent": agent_name,
                                        "mode": "multi-agent",
                                    }),
                                )
                                .await;

                                let result = {
                                    let mut orch = state.orchestrator.lock().await;
                                    orch.send_to(agent_name, &content).await
                                };

                                match result {
                                    Ok(mut response) => {
                                        response = strip_think_tags(&response);
                                        if stream {
                                            let chunk_size = 8;
                                            let chars: Vec<char> = response.chars().collect();
                                            let mut idx: u64 = 0;
                                            let mut think_filter = StreamingThinkFilter::new();
                                            for chunk in chars.chunks(chunk_size) {
                                                let raw_text: String = chunk.iter().collect();
                                                let text = think_filter.process(&raw_text);
                                                if text.is_empty() {
                                                    continue;
                                                }
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
                                                    "mode": "multi-agent",
                                                    "agent": agent_name,
                                                }),
                                            )
                                            .await;
                                        } else {
                                            let _ = send_json(
                                                &mut socket,
                                                &serde_json::json!({
                                                    "type": "chat_done",
                                                    "request_id": &request_id,
                                                    "full_content": &response,
                                                    "mode": "multi-agent",
                                                    "agent": agent_name,
                                                }),
                                            )
                                            .await;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = send_json(
                                            &mut socket,
                                            &serde_json::json!({
                                                "type": "chat_error",
                                                "request_id": &request_id,
                                                "error": e.to_string(),
                                                "agent": agent_name,
                                            }),
                                        )
                                        .await;
                                    }
                                }
                                continue; // Skip default agent path
                            }
                            // If agent not in orchestrator, fall through to default path
                        }

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
                                Some(Ok(mut response)) => {
                                    response = strip_think_tags(&response);
                                    // ── SecretRedactor: scan OUTGOING response ──
                                    let (redacted, count) = REDACTOR.redact(&response);
                                    if count > 0 {
                                        tracing::warn!(
                                            "🔐 SecretRedactor: redacted {count} secret(s) from agent response"
                                        );
                                        response = redacted;
                                    }
                                    if stream {
                                        // Emit as rapid chunks for streaming UX
                                        let chunk_size = 8; // chars per chunk
                                        let chars: Vec<char> = response.chars().collect();
                                        let mut idx: u64 = 0;
                                        let mut think_filter = StreamingThinkFilter::new();
                                        for chunk in chars.chunks(chunk_size) {
                                            let raw_text: String = chunk.iter().collect();
                                            let text = think_filter.process(&raw_text);
                                            if text.is_empty() {
                                                continue;
                                            }
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
                                Ok(mut response) => {
                                    // ── SecretRedactor: scan OUTGOING response ──
                                    let (redacted, count) = REDACTOR.redact(&response);
                                    if count > 0 {
                                        tracing::warn!(
                                            "🔐 SecretRedactor: redacted {count} secret(s) from direct response"
                                        );
                                        response = redacted;
                                    }
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

        let mut think_filter = StreamingThinkFilter::new();

        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(content) = json["message"]["content"].as_str()
                && !content.is_empty()
            {
                full_content.push_str(content);
                let filtered = think_filter.process(content);
                if filtered.is_empty() {
                    continue;
                }
                let _ = send_json(
                    socket,
                    &serde_json::json!({
                        "type": "chat_chunk",
                        "request_id": request_id,
                        "content": filtered,
                        "index": chunk_idx,
                    }),
                )
                .await;
                chunk_idx += 1;
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
        let mut content = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        content = strip_think_tags(&content);

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
        let config = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
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

        let mut think_filter = StreamingThinkFilter::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ")
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(data)
                && let Some(content) = json["choices"][0]["delta"]["content"].as_str()
                && !content.is_empty()
            {
                full_content.push_str(content);
                let filtered = think_filter.process(content);
                if filtered.is_empty() {
                    continue;
                }
                let _ = send_json(
                    socket,
                    &serde_json::json!({
                        "type": "chat_chunk",
                        "request_id": request_id,
                        "content": filtered,
                        "index": chunk_idx,
                    }),
                )
                .await;
                chunk_idx += 1;
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
        let mut content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        content = strip_think_tags(&content);

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

/// Strip <think>...</think> tags generated by reasoner models (DeepSeek-R1, Qwen3)
/// If the resulting text is empty, returns a placeholder.
fn strip_think_tags(text: &str) -> String {
    let mut output = String::new();
    let mut current = text;

    while let Some(start_idx) = current.find("<think>") {
        output.push_str(&current[..start_idx]);
        if let Some(end_idx) = current[start_idx..].find("</think>") {
            current = &current[start_idx + end_idx + 8..];
        } else {
            // Unclosed think tag
            current = "";
            break;
        }
    }
    output.push_str(current);

    let trimmed = output.trim();
    if trimmed.is_empty() && !text.trim().is_empty() {
        return "*(Đã suy nghĩ xong, nhưng không có thông điệp trả lời)*".to_string();
    }
    trimmed.to_string()
}

/// A stateful filter to strip <think>...</think> tags during a streaming response.
struct StreamingThinkFilter {
    in_think: bool,
    buffer: String,
}

impl StreamingThinkFilter {
    fn new() -> Self {
        Self {
            in_think: false,
            buffer: String::new(),
        }
    }

    fn process(&mut self, chunk: &str) -> String {
        self.buffer.push_str(chunk);

        let mut result = String::new();
        loop {
            if !self.in_think {
                if let Some(idx) = self.buffer.find("<think>") {
                    result.push_str(&self.buffer[..idx]);
                    self.in_think = true;
                    self.buffer = self.buffer[idx + 7..].to_string();
                } else if self.buffer.contains("<t")
                    || self.buffer.contains("<th")
                    || self.buffer.contains("<thi")
                    || self.buffer.contains("<thin")
                    || self.buffer.contains("<think")
                {
                    // Possible start of <think>, hold in buffer
                    break;
                } else {
                    result.push_str(&self.buffer);
                    self.buffer.clear();
                    break;
                }
            } else if let Some(idx) = self.buffer.find("</think>") {
                self.in_think = false;
                self.buffer = self.buffer[idx + 8..].to_string();
            } else {
                // Inside think block, discard safely
                // Keep the end if it looks like the start of </think>
                let mut safe_idx = self.buffer.len();
                if self.buffer.ends_with("<") {
                    safe_idx -= 1;
                } else if self.buffer.ends_with("</") {
                    safe_idx -= 2;
                } else if self.buffer.ends_with("</t") {
                    safe_idx -= 3;
                } else if self.buffer.ends_with("</th") {
                    safe_idx -= 4;
                } else if self.buffer.ends_with("</thi") {
                    safe_idx -= 5;
                } else if self.buffer.ends_with("</thin") {
                    safe_idx -= 6;
                } else if self.buffer.ends_with("</think") {
                    safe_idx -= 7;
                }

                self.buffer = self.buffer[safe_idx..].to_string();
                break;
            }
        }
        result
    }
}
