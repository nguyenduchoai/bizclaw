//! Claude Web Provider — API-based session using cookies.
//!
//! Claude's web interface uses a REST API at `https://claude.ai/api/`.
//! We use the session cookie (`sessionKey`) to authenticate requests.
//! Responses are streamed via SSE.

use async_trait::async_trait;

use crate::cdp::CdpClient;
use crate::cookie_auth;
use crate::types::{AuthCheckResult, WebAuthModel};

use super::WebProvider;

const LOG_TAG: &str = "[Claude-Web]";
const API_BASE: &str = "https://claude.ai/api";

pub struct ClaudeWebProvider {
    models: Vec<WebAuthModel>,
}

impl Default for ClaudeWebProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeWebProvider {
    pub fn new() -> Self {
        Self {
            models: vec![
                WebAuthModel {
                    id: "webauth-claude-sonnet".to_string(),
                    name: "Claude Sonnet (WebAuth)".to_string(),
                    context_window: 200000,
                },
                WebAuthModel {
                    id: "webauth-claude-haiku".to_string(),
                    name: "Claude Haiku (WebAuth)".to_string(),
                    context_window: 200000,
                },
            ],
        }
    }

    /// Get organization ID for the session.
    async fn get_org_id(&self, cdp: &CdpClient) -> Result<String, String> {
        let js = format!(
            r#"
            (async () => {{
                try {{
                    const resp = await fetch('{API_BASE}/organizations', {{
                        credentials: 'include',
                        headers: {{ 
                            'Content-Type': 'application/json',
                            'anthropic-client-platform': 'web_claude_ai'
                        }}
                    }});
                    const data = await resp.json();
                    if (Array.isArray(data) && data.length > 0) {{
                        return data[0].uuid;
                    }}
                    return null;
                }} catch (e) {{
                    return 'ERROR:' + e.message;
                }}
            }})()
            "#
        );
        let result = cdp.evaluate_js(&js).await?;
        result
            .as_str()
            .filter(|s| !s.starts_with("ERROR:") && !s.is_empty())
            .map(String::from)
            .ok_or_else(|| format!("{} Could not get org ID", LOG_TAG))
    }

    /// Create a new conversation and send a message.
    async fn send_message(
        &self,
        cdp: &CdpClient,
        org_id: &str,
        prompt: &str,
    ) -> Result<String, String> {
        let escaped_prompt = prompt
            .replace('\\', "\\\\")
            .replace('\"', "\\\"")
            .replace('\n', "\\n");

        // Create conversation and send message
        let js = format!(
            r#"
            (async () => {{
                try {{
                    // Create conversation
                    const createResp = await fetch('{API_BASE}/organizations/{org_id}/chat_conversations', {{
                        method: 'POST',
                        credentials: 'include',
                        headers: {{ 
                            'Content-Type': 'application/json',
                            'anthropic-client-platform': 'web_claude_ai'
                        }},
                        body: JSON.stringify({{ name: '', model: 'claude-sonnet-4-20250514' }})
                    }});
                    const conv = await createResp.json();
                    const convId = conv.uuid;
                    if (!convId) return 'ERROR:No conversation UUID';

                    // Send message (non-streaming for simplicity)
                    const chatResp = await fetch(`{API_BASE}/organizations/{org_id}/chat_conversations/${{convId}}/completion`, {{
                        method: 'POST',
                        credentials: 'include',
                        headers: {{ 
                            'Content-Type': 'application/json', 
                            'Accept': 'text/event-stream',
                            'anthropic-client-platform': 'web_claude_ai'
                        }},
                        body: JSON.stringify({{
                            prompt: "{escaped_prompt}",
                            timezone: 'Asia/Ho_Chi_Minh',
                            model: 'claude-sonnet-4-20250514'
                        }})
                    }});

                    const text = await chatResp.text();

                    // Parse SSE: each line is "data: {{...}}\n"
                    let fullText = '';
                    for (const line of text.split('\\n')) {{
                        if (line.startsWith('data: ')) {{
                            try {{
                                const d = JSON.parse(line.slice(6));
                                if (d.completion) fullText += d.completion;
                                if (d.type === 'content_block_delta' && d.delta?.text) {{
                                    fullText += d.delta.text;
                                }}
                            }} catch (e) {{}}
                        }}
                    }}

                    // Cleanup: delete conversation
                    fetch(`{API_BASE}/organizations/{org_id}/chat_conversations/${{convId}}`, {{
                        method: 'DELETE',
                        credentials: 'include'
                    }}).catch(() => {{}});

                    return fullText || 'ERROR:Empty response';
                }} catch (e) {{
                    return 'ERROR:' + e.message;
                }}
            }})()
            "#
        );

        let result = cdp.evaluate_js(&js).await?;
        let text = result
            .as_str()
            .ok_or_else(|| format!("{} Response is not a string", LOG_TAG))?;

        if text.starts_with("ERROR:") {
            return Err(format!("{} {}", LOG_TAG, text));
        }

        Ok(text.to_string())
    }
}

#[async_trait]
impl WebProvider for ClaudeWebProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn name(&self) -> &str {
        "Claude Web"
    }

    fn login_url(&self) -> &str {
        "https://claude.ai"
    }

    fn models(&self) -> &[WebAuthModel] {
        &self.models
    }

    async fn check_auth(&self, cdp: &CdpClient) -> AuthCheckResult {
        cookie_auth::check_provider_auth(cdp, "claude", self.login_url()).await
    }

    async fn chat(&self, cdp: &CdpClient, prompt: &str) -> Result<String, String> {
        let org_id = self.get_org_id(cdp).await?;
        self.send_message(cdp, &org_id, prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let p = ClaudeWebProvider::new();
        assert_eq!(p.id(), "claude");
        assert_eq!(p.login_url(), "https://claude.ai");
        assert!(p.models().iter().any(|m| m.id == "webauth-claude-sonnet"));
    }
}
