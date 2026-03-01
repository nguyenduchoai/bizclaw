//! SSE (Server-Sent Events) transport for MCP — connects to MCP servers via HTTP SSE.
//!
//! This enables connecting to MCP servers that expose an SSE endpoint
//! instead of using stdio. Useful for remote MCP servers.

use std::collections::HashMap;

use crate::types::{JsonRpcRequest, JsonRpcResponse};

/// SSE transport — connects to an MCP server via HTTP Server-Sent Events.
pub struct SseTransport {
    /// The SSE endpoint URL.
    endpoint: String,
    /// HTTP client for making requests.
    client: reqwest::Client,
    /// Additional headers.
    headers: HashMap<String, String>,
    /// Connected state.
    connected: bool,
}

impl SseTransport {
    /// Create a new SSE transport.
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: reqwest::Client::new(),
            headers: HashMap::new(),
            connected: false,
        }
    }

    /// Set an authorization header.
    pub fn with_auth(mut self, token: &str) -> Self {
        self.headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }

    /// Add a custom header.
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Connect and initialize the session.
    pub async fn connect(&mut self) -> Result<(), String> {
        // Send an initialize request to verify connectivity
        let test_req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 0,
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "bizclaw-mcp",
                    "version": "0.3.0"
                }
            })),
        };

        self.request(&test_req).await?;
        self.connected = true;
        tracing::info!("🌐 SSE transport connected to {}", self.endpoint);
        Ok(())
    }

    /// Send a JSON-RPC request via HTTP POST and read the response.
    pub async fn request(&self, req: &JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        let mut builder = self.client.post(&self.endpoint);

        for (key, value) in &self.headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        let response: reqwest::Response = builder
            .json(req)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("SSE request error: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "SSE server returned status {}",
                response.status()
            ));
        }

        let body: String = response
            .text()
            .await
            .map_err(|e| format!("SSE response read error: {e}"))?;

        // Parse SSE data lines — look for data: {...} lines
        for raw_line in body.lines() {
            let trimmed = raw_line.trim();
            if let Some(data) = trimmed.strip_prefix("data: ") {
                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(data) {
                    return Ok(resp);
                }
            }
            // Also try parsing the whole body as JSON (non-SSE response)
            if trimmed.starts_with('{') {
                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(trimmed) {
                    return Ok(resp);
                }
            }
        }

        // Try parsing the entire body as JSON
        let truncated = &body[..body.len().min(200)];
        serde_json::from_str::<JsonRpcResponse>(&body)
            .map_err(|e| format!("SSE parse error: {e} — body: {truncated}"))
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get the endpoint URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Disconnect.
    pub fn disconnect(&mut self) {
        self.connected = false;
    }
}

/// Streamable HTTP transport for MCP — uses standard HTTP POST.
/// 
/// This is the simplest transport: just POST JSON-RPC to an HTTP endpoint.
pub struct HttpTransport {
    /// The HTTP endpoint URL.
    endpoint: String,
    /// HTTP client.
    client: reqwest::Client,
    /// Additional headers.
    headers: HashMap<String, String>,
}

impl HttpTransport {
    /// Create a new HTTP transport.
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: reqwest::Client::new(),
            headers: HashMap::new(),
        }
    }

    /// Set authorization.
    pub fn with_auth(mut self, token: &str) -> Self {
        self.headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }

    /// Add a custom header.
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Send a JSON-RPC request and get the response.
    pub async fn request(&self, req: &JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        let mut builder = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json");

        for (key, value) in &self.headers {
            builder = builder.header(key.as_str(), value.as_str());
        }

        let response: reqwest::Response = builder
            .json(req)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("HTTP request error: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body: String = response.text().await.unwrap_or_default();
            let truncated = &body[..body.len().min(500)];
            return Err(format!("HTTP {} — {}", status, truncated));
        }

        let body: String = response
            .text()
            .await
            .map_err(|e| format!("HTTP response read error: {e}"))?;

        serde_json::from_str::<JsonRpcResponse>(&body)
            .map_err(|e| format!("HTTP response parse error: {e}"))
    }

    /// Get the endpoint URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

/// Transport type enum for dynamic dispatch.
pub enum McpTransport {
    /// Stdio (existing).
    Stdio(super::transport::StdioTransport),
    /// SSE (new).
    Sse(SseTransport),
    /// HTTP (new).
    Http(HttpTransport),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_transport_creation() {
        let transport = SseTransport::new("http://localhost:8080/mcp")
            .with_auth("test-token")
            .with_header("X-Custom", "value");

        assert_eq!(transport.endpoint(), "http://localhost:8080/mcp");
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_http_transport_creation() {
        let transport = HttpTransport::new("http://localhost:3000/v1/mcp")
            .with_auth("sk-test");

        assert_eq!(transport.endpoint(), "http://localhost:3000/v1/mcp");
    }
}
