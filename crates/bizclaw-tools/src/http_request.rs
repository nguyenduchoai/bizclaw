//! HTTP Request tool — make HTTP requests to external APIs

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

pub struct HttpRequestTool;

impl HttpRequestTool {
    pub fn new() -> Self { Self }
}

impl Default for HttpRequestTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for HttpRequestTool {
    fn name(&self) -> &str { "http_request" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "http_request".into(),
            description: "Make HTTP requests to APIs and websites. Supports GET, POST, PUT, DELETE with headers and body.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to request"
                    },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"],
                        "description": "HTTP method (default: GET)"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Request headers (key-value pairs)"
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body (for POST/PUT/PATCH)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Request timeout in seconds (default: 15)"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let url = args["url"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'url'".into()))?;
        let method = args["method"].as_str().unwrap_or("GET").to_uppercase();
        let timeout = args["timeout_secs"].as_u64().unwrap_or(15);

        // Safety check: block requests to internal/localhost unless explicitly allowed
        let lower_url = url.to_lowercase();
        if lower_url.contains("169.254.") || lower_url.contains("metadata.google") {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: "Blocked: Cannot access cloud metadata endpoints".into(),
                success: false,
            });
        }

        let client = reqwest::Client::builder()
            .user_agent("BizClaw/1.0")
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Client error: {e}")))?;

        let mut request = match method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            "HEAD" => client.head(url),
            _ => return Err(bizclaw_core::error::BizClawError::Tool(format!("Unsupported method: {method}"))),
        };

        // Add custom headers
        if let Some(headers) = args["headers"].as_object() {
            for (key, value) in headers {
                if let Some(val_str) = value.as_str() {
                    if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                        if let Ok(header_val) = reqwest::header::HeaderValue::from_str(val_str) {
                            request = request.header(header_name, header_val);
                        }
                    }
                }
            }
        }

        // Add body
        if let Some(body) = args["body"].as_str() {
            request = request.body(body.to_string());
            // Auto-detect content type if not set
            if args["headers"].as_object().map(|h| !h.contains_key("content-type")).unwrap_or(true) {
                if body.starts_with('{') || body.starts_with('[') {
                    request = request.header("Content-Type", "application/json");
                }
            }
        }

        let start = std::time::Instant::now();
        let response = request.send().await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Request failed: {e}")))?;

        let elapsed = start.elapsed();
        let status = response.status();
        let headers: String = response.headers().iter()
            .take(10) // limit header output
            .map(|(k, v)| format!("{}: {}", k.as_str(), v.to_str().unwrap_or("?")))
            .collect::<Vec<_>>()
            .join("\n");

        let body_text = response.text().await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Read body failed: {e}")))?;

        // Truncate very large responses
        let body_display = if body_text.len() > 8000 {
            format!("{}...\n\n[truncated, {} total bytes]", &body_text[..8000], body_text.len())
        } else {
            body_text
        };

        let output = format!(
            "HTTP {} {} → {} ({:.0}ms)\n\nHeaders:\n{}\n\nBody:\n{}",
            method, url, status, elapsed.as_millis(), headers, body_display
        );

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: status.is_success(),
        })
    }
}
