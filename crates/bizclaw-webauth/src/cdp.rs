//! Chrome DevTools Protocol (CDP) client.
//!
//! Communicates with Chrome/Chromium via WebSocket for:
//! - Template capture (Fetch.requestPaused)
//! - JavaScript execution (Runtime.evaluate)
//! - Cookie management (Network.getCookies)
//! - Page navigation (Page.navigate)

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing;

const LOG_TAG: &str = "[CDP]";

/// CDP client for communicating with a Chrome/Chromium instance.
pub struct CdpClient {
    /// WebSocket sender
    sender: Arc<
        Mutex<
            futures::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                WsMessage,
            >,
        >,
    >,
    /// Pending responses (method call ID → response sender)
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    /// Event listeners (method name → sender)
    event_listeners: Arc<Mutex<HashMap<String, Vec<mpsc::UnboundedSender<Value>>>>>,
    /// Next message ID
    next_id: Arc<Mutex<u64>>,
    /// Reader task handle
    _reader_handle: tokio::task::JoinHandle<()>,
}

impl CdpClient {
    /// Connect to a Chrome/Chromium instance via its DevTools WebSocket URL.
    ///
    /// To get the URL, launch Chrome with `--remote-debugging-port=9222`
    /// and query `http://127.0.0.1:9222/json/version` for `webSocketDebuggerUrl`.
    pub async fn connect(ws_url: &str) -> Result<Self, String> {
        tracing::info!("{} Connecting to {}", LOG_TAG, ws_url);

        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| format!("{} WebSocket connect failed: {}", LOG_TAG, e))?;

        let (sender, mut receiver) = ws_stream.split();
        let sender = Arc::new(Mutex::new(sender));
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let event_listeners: Arc<Mutex<HashMap<String, Vec<mpsc::UnboundedSender<Value>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_clone = pending.clone();
        let events_clone = event_listeners.clone();

        // Background task: read WebSocket messages and dispatch
        let reader_handle = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let WsMessage::Text(text) = msg
                    && let Ok(json) = serde_json::from_str::<Value>(&text)
                {
                    if let Some(id) = json.get("id").and_then(|v| v.as_u64()) {
                        // Method response
                        let mut pending = pending_clone.lock().await;
                        if let Some(tx) = pending.remove(&id) {
                            let _ = tx.send(json);
                        }
                    } else if let Some(method) = json.get("method").and_then(|v| v.as_str()) {
                        // Event
                        let listeners = events_clone.lock().await;
                        if let Some(senders) = listeners.get(method) {
                            let params = json
                                .get("params")
                                .cloned()
                                .unwrap_or(Value::Object(serde_json::Map::new()));
                            for tx in senders {
                                let _ = tx.send(params.clone());
                            }
                        }
                    }
                }
            }
            tracing::warn!("{} WebSocket reader exited", LOG_TAG);
        });

        tracing::info!("{} Connected successfully", LOG_TAG);

        Ok(Self {
            sender,
            pending,
            event_listeners,
            next_id: Arc::new(Mutex::new(1)),
            _reader_handle: reader_handle,
        })
    }

    /// Send a CDP command and wait for the response.
    pub async fn send_command(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = {
            let mut next = self.next_id.lock().await;
            let id = *next;
            *next += 1;
            id
        };

        let msg = json!({
            "id": id,
            "method": method,
            "params": params,
        });

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        {
            let mut sender = self.sender.lock().await;
            sender
                .send(WsMessage::Text(msg.to_string()))
                .await
                .map_err(|e| format!("{} Send failed: {}", LOG_TAG, e))?;
        }

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => {
                if let Some(error) = response.get("error") {
                    Err(format!("{} {} error: {}", LOG_TAG, method, error))
                } else {
                    Ok(response
                        .get("result")
                        .cloned()
                        .unwrap_or(Value::Object(serde_json::Map::new())))
                }
            }
            Ok(Err(_)) => Err(format!("{} {} response channel dropped", LOG_TAG, method)),
            Err(_) => Err(format!("{} {} timed out (30s)", LOG_TAG, method)),
        }
    }

    /// Subscribe to a CDP event. Returns a receiver that yields event params.
    pub async fn subscribe(&self, event: &str) -> mpsc::UnboundedReceiver<Value> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut listeners = self.event_listeners.lock().await;
        listeners.entry(event.to_string()).or_default().push(tx);
        rx
    }

    /// Wait for a single occurrence of a CDP event with timeout.
    pub async fn wait_for_event(&self, event: &str, timeout_ms: u64) -> Result<Value, String> {
        let mut rx = self.subscribe(event).await;
        match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), rx.recv()).await {
            Ok(Some(params)) => Ok(params),
            Ok(None) => Err(format!("{} Event channel closed for {}", LOG_TAG, event)),
            Err(_) => Err(format!(
                "{} Timed out waiting for {} ({}ms)",
                LOG_TAG, event, timeout_ms
            )),
        }
    }

    // ─── High-Level Helpers ────────────────────────────────────────────

    /// Execute JavaScript in the page context and return the result.
    pub async fn evaluate_js(&self, expression: &str) -> Result<Value, String> {
        let result = self
            .send_command(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "awaitPromise": true,
                    "returnByValue": true,
                }),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            return Err(format!(
                "{} JS exception: {}",
                LOG_TAG,
                exception
                    .get("exception")
                    .and_then(|e| e.get("description"))
                    .and_then(|d| d.as_str())
                    .unwrap_or("unknown")
            ));
        }

        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    /// Navigate to a URL.
    pub async fn navigate(&self, url: &str) -> Result<(), String> {
        self.send_command("Page.navigate", json!({ "url": url }))
            .await?;
        Ok(())
    }

    /// Get all cookies for a URL.
    pub async fn get_cookies(&self, urls: &[&str]) -> Result<Vec<Value>, String> {
        let result = self
            .send_command("Network.getCookies", json!({ "urls": urls }))
            .await?;
        Ok(result
            .get("cookies")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default())
    }

    /// Enable Fetch domain for request interception.
    pub async fn enable_fetch(&self, url_pattern: &str) -> Result<(), String> {
        self.send_command(
            "Fetch.enable",
            json!({
                "patterns": [{
                    "urlPattern": url_pattern,
                    "requestStage": "Request"
                }]
            }),
        )
        .await?;
        Ok(())
    }

    /// Disable Fetch domain.
    pub async fn disable_fetch(&self) -> Result<(), String> {
        self.send_command("Fetch.disable", json!({})).await?;
        Ok(())
    }

    /// Continue a paused request.
    pub async fn continue_request(&self, request_id: &str) -> Result<(), String> {
        self.send_command("Fetch.continueRequest", json!({ "requestId": request_id }))
            .await?;
        Ok(())
    }

    /// Type text simulating human typing (Jitter and Delays) to prevent bot detection.
    /// It sends individual Input.dispatchKeyEvent for each character.
    pub async fn type_text_human_like(&self, text: &str) -> Result<(), String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        for c in text.chars() {
            // Send char as key down & key up
            self.send_command(
                "Input.dispatchKeyEvent",
                json!({
                    "type": "char",
                    "text": c.to_string(),
                }),
            )
            .await?;

            // Simulating human delay between 50ms and 150ms per keystroke.
            let delay_ms = rng.gen_range(50..150);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

            // Occasionally take a longer pause (thinking time, random 2% chance)
            if rng.gen_bool(0.02) {
                let thinking_time = rng.gen_range(400..1200);
                tracing::debug!("{} Humanizer: pausing for {} ms", LOG_TAG, thinking_time);
                tokio::time::sleep(std::time::Duration::from_millis(thinking_time)).await;
            }
        }
        Ok(())
    }

    /// Get the current page URL.
    pub async fn get_url(&self) -> Result<String, String> {
        let result = self.evaluate_js("window.location.href").await?;
        result
            .as_str()
            .map(String::from)
            .ok_or_else(|| format!("{} Could not get URL", LOG_TAG))
    }
}

/// Find the WebSocket debugger URL for a running Chrome instance.
///
/// Chrome must be started with `--remote-debugging-port=PORT`.
pub async fn find_chrome_ws_url(port: u16) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{}/json/version", port);
    let resp = reqwest::get(&url).await.map_err(|e| {
        format!(
            "Could not connect to Chrome debugger on port {}: {}",
            port, e
        )
    })?;

    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid JSON from Chrome debugger: {}", e))?;

    json.get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "No webSocketDebuggerUrl in Chrome response".to_string())
}

/// Find WebSocket URLs for all open tabs/pages.
pub async fn find_page_ws_urls(port: u16) -> Result<Vec<(String, String)>, String> {
    let url = format!("http://127.0.0.1:{}/json", port);
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("Could not connect to Chrome debugger: {}", e))?;

    let json: Vec<Value> = resp
        .json()
        .await
        .map_err(|e| format!("Invalid JSON from Chrome: {}", e))?;

    Ok(json
        .iter()
        .filter_map(|page| {
            let ws_url = page.get("webSocketDebuggerUrl")?.as_str()?;
            let page_url = page.get("url")?.as_str()?;
            Some((page_url.to_string(), ws_url.to_string()))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types() {
        // Basic type checks
        let _msg = json!({
            "id": 1,
            "method": "Runtime.evaluate",
            "params": { "expression": "1+1" }
        });
    }
}
