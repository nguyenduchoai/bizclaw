use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex as TokioMutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};
use url::Url;

use crate::error::BrowserError;

#[derive(Clone)]
pub struct CdpClient {
    sender: Arc<TokioMutex<Option<mpsc::Sender<CdpCommand>>>>,
}

#[derive(Debug, Serialize)]
struct WireCommand {
    id: i32,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug)]
pub struct CdpCommand {
    pub id: i32,
    pub method: String,
    pub params: Option<Value>,
    #[allow(dead_code)]
    pub response_tx: oneshot::Sender<CdpResponse>,
}

impl CdpCommand {
    fn to_wire(&self) -> WireCommand {
        WireCommand {
            id: self.id,
            method: self.method.clone(),
            params: self.params.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpResponse {
    pub id: i32,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<CdpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpEvent {
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl CdpClient {
    pub async fn connect(ws_url: &str) -> Result<Self, BrowserError> {
        info!("Connecting to CDP WebSocket: {}", ws_url);
        
        let _ = Url::parse(ws_url)
            .map_err(|e| BrowserError::ConnectionFailed(e.to_string()))?;
        
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| BrowserError::ConnectionFailed(e.to_string()))?;
        
        let (mut write, mut read) = ws_stream.split();
        
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<CdpCommand>(100);
        
        let sender = Arc::new(TokioMutex::new(Some(cmd_tx.clone())));
        
        tokio::spawn(async move {
            let mut pending_commands: HashMap<i32, oneshot::Sender<CdpResponse>> = HashMap::new();
            
            loop {
                tokio::select! {
                    cmd = cmd_rx.recv() => {
                        if let Some(cmd) = cmd {
                            let cmd_id = cmd.id;
                            let cmd_method = cmd.method.clone();
                            let wire_cmd = cmd.to_wire();
                            let msg = serde_json::to_string(&wire_cmd)
                                .unwrap_or_default();
                            pending_commands.insert(cmd_id, cmd.response_tx);
                            if write.send(Message::Text(msg)).await.is_err() {
                                error!("Failed to send CDP command");
                                break;
                            }
                            debug!("Sent CDP command: {}", cmd_method);
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                    if let Some(id) = value.get("id").and_then(|i| i.as_i64()) {
                                        if let Some(tx) = pending_commands.remove(&(id as i32)) {
                                            let resp: CdpResponse = serde_json::from_value(value.clone())
                                                .unwrap_or_else(|_| CdpResponse {
                                                    id: id as i32,
                                                    result: None,
                                                    error: Some(CdpError {
                                                        code: -1,
                                                        message: "Parse error".to_string(),
                                                    }),
                                                });
                                            let _ = tx.send(resp);
                                        }
                                    } else if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                                        let event = CdpEvent {
                                            method: method.to_string(),
                                            params: value.get("params").cloned(),
                                        };
                                        debug!("Received CDP event: {}", method);
                                        drop(event);
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                info!("CDP WebSocket closed");
                                break;
                            }
                            Some(Err(e)) => {
                                error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            for (_, tx) in pending_commands {
                let _ = tx.send(CdpResponse {
                    id: -1,
                    result: None,
                    error: Some(CdpError {
                        code: -1,
                        message: "Connection closed".to_string(),
                    }),
                });
            }
        });
        
        Ok(Self {
            sender,
        })
    }
    
    pub async fn new_tab(port: u16) -> Result<(Self, String), BrowserError> {
        info!("Creating new tab via Chrome DevTools on port {}", port);
        
        let client = Self::connect(&format!("ws://localhost:{}/devtools/browser", port)).await?;
        
        let response = client.send_command("Target.createTarget", Some(serde_json::json!({
            "url": "about:blank"
        }))).await?;
        
        let page_id = response.get("targetId")
            .and_then(|t| t.as_str())
            .ok_or_else(|| BrowserError::ConnectionFailed("Failed to create new tab".to_string()))?
            .to_string();
        
        let ws_url = format!("ws://localhost:{}/devtools/page/{}", port, page_id);
        let page_client = Self::connect(&ws_url).await?;
        
        info!("New tab created with ID: {}", page_id);
        Ok((page_client, page_id))
    }
    
    pub async fn send_command(&self, method: &str, params: Option<Value>) -> Result<Value, BrowserError> {
        let sender = self.sender.lock().await;
        let sender = sender.as_ref()
            .ok_or_else(|| BrowserError::ConnectionFailed("Client not connected".to_string()))?;
        
        let (response_tx, response_rx) = oneshot::channel::<CdpResponse>();
        let id = rand_id();
        
        let cmd = CdpCommand {
            id,
            method: method.to_string(),
            params,
            response_tx,
        };
        
        sender.send(cmd).await
            .map_err(|_| BrowserError::ConnectionFailed("Failed to send command".to_string()))?;
        
        let response = response_rx.await
            .map_err(|_| BrowserError::Timeout("Command timeout".to_string()))?;
        
        match response.error {
            Some(e) => Err(BrowserError::CdpError(e.message)),
            None => Ok(response.result.unwrap_or(Value::Null)),
        }
    }
    
    pub async fn page_enable(&self) -> std::result::Result<(), BrowserError> {
        self.send_command("Page.enable", None).await?;
        self.send_command("Runtime.enable", None).await?;
        Ok(())
    }
    
    pub async fn navigate(&self, url: &str) -> Result<String, BrowserError> {
        let params = serde_json::json!({
            "url": url,
            "transitionType": "typed"
        });
        let result = self.send_command("Page.navigate", Some(params)).await?;
        
        let frame_id = result.get("frameId")
            .and_then(|f| f.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        Ok(frame_id)
    }
    
    pub async fn reload(&self, ignore_cache: bool) -> Result<()> {
        let params = serde_json::json!({
            "ignoreCache": ignore_cache
        });
        self.send_command("Page.reload", Some(params)).await?;
        Ok(())
    }
}

fn rand_id() -> i32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (dur.as_nanos() % i32::MAX as u128) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cdp_client_creation() {
        let client = CdpClient {
            sender: Arc::new(TokioMutex::new(None)),
        };
        
        assert!(client.sender.try_lock().is_ok());
    }
}
