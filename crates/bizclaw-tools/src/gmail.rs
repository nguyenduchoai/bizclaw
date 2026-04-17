//! Read and parse incoming emails via IMAP, focused on AI summarization.

use bizclaw_core::error::BizClawError;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use native_tls::TlsConnector;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GmailToolRequest {
    pub folder: Option<String>,
    pub max_emails: Option<usize>,
}

pub struct GmailTool;

impl GmailTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GmailTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for GmailTool {
    fn name(&self) -> &str {
        "gmail_reader"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "gmail_reader".into(),
            description: "Read unread emails via IMAP. Returns a markdown summary of their content. Uses credentials from system config. Requires folder (optional, default INBOX) and max_emails limit.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "folder": { "type": "string", "description": "Optional Mail folder, default INBOX" },
                    "max_emails": { "type": "integer", "description": "Max emails to summary, default 5" }
                }
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult, BizClawError> {
        let req: GmailToolRequest =
            serde_json::from_str(arguments).unwrap_or_else(|_| GmailToolRequest {
                folder: None,
                max_emails: None,
            });

        let folder = req.folder.unwrap_or_else(|| "INBOX".to_string());
        let limit = req.max_emails.unwrap_or(5);

        // Load credentials from system config
        let cfg_path = std::env::var("BIZCLAW_CONFIG").unwrap_or_else(|_| "config.toml".into());
        let full_cfg =
            bizclaw_core::config::BizClawConfig::load_from(std::path::Path::new(&cfg_path))
                .unwrap_or_default();

        if full_cfg.channel.email.is_empty() || !full_cfg.channel.email[0].enabled {
            return Err(BizClawError::Tool(
                "Email channel is not configured or disabled in config.toml".into(),
            ));
        }

        let email_cfg = &full_cfg.channel.email[0];
        let host = email_cfg.imap_host.clone();
        let username = email_cfg.email.clone();
        let password = email_cfg.password.clone();
        let port = email_cfg.imap_port;

        if host.is_empty() || username.is_empty() {
            return Err(BizClawError::Tool(
                "IMAP host or email is not configured".into(),
            ));
        }

        let result = tokio::task::spawn_blocking(move || {
            let tls = TlsConnector::builder()
                .build()
                .map_err(|e| format!("TLS Error: {}", e))?;
            let client = imap::connect((host.as_str(), port), host.as_str(), &tls)
                .map_err(|e| format!("IMAP Connect Error: {}", e))?;
            let mut imap_session = client
                .login(username, password)
                .map_err(|e| format!("Login failed: {}", e.0))?;

            imap_session
                .select(&folder)
                .map_err(|e| format!("Folder error: {}", e))?;

            // Fetch UNSEEN messages first to avoid reading the whole inbox
            let uids = imap_session
                .uid_search("UNSEEN")
                .map_err(|e| format!("Search error: {}", e))?;

            let mut uids: Vec<_> = uids.into_iter().collect();
            uids.sort_by(|a: &u32, b: &u32| b.cmp(a)); // Newest first
            uids.truncate(limit);

            if uids.is_empty() {
                let _ = imap_session.logout();
                return Ok::<String, String>("No unread emails found.".into());
            }

            let uid_set = uids
                .iter()
                .map(|u: &u32| u.to_string())
                .collect::<Vec<_>>()
                .join(",");

            // Fetch headers and body for matched UIDs
            let fetches = imap_session
                .uid_fetch(
                    &uid_set,
                    "(UID RFC822.SIZE BODY[HEADER.FIELDS (SUBJECT FROM DATE)] BODY[TEXT])",
                )
                .map_err(|e| format!("Fetch error: {}", e))?;

            let mut reports = Vec::new();
            for m in fetches.iter() {
                let header = if let Some(h) = m.header() {
                    String::from_utf8_lossy(h).to_string()
                } else {
                    "No Header".to_string()
                };
                let body = if let Some(b) = m.text() {
                    String::from_utf8_lossy(b).to_string()
                } else {
                    "No Body".to_string()
                };

                let body_preview = if body.len() > 1000 {
                    format!("{}...", &body[..1000])
                } else {
                    body.clone()
                };

                reports.push(format!(
                    "### Email UID: {}\n**Headers:**\n{}\n**Content Preview:**\n{}\n---\n",
                    m.uid.unwrap_or(0),
                    header.trim(),
                    body_preview.replace("\r", "")
                ));
            }

            let _ = imap_session.logout();
            Ok::<String, String>(reports.join("\n"))
        })
        .await;

        match result {
            Ok(Ok(text)) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: text,
                success: true,
            }),
            Ok(Err(e)) => Err(BizClawError::Tool(e)),
            Err(e) => Err(BizClawError::Tool(e.to_string())),
        }
    }
}
