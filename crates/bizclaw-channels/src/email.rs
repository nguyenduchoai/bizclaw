//! Email Channel — IMAP polling + SMTP sending.
//!
//! Reads emails via IMAP (sync, in spawn_blocking), routes them to the AI agent,
//! and sends replies via SMTP (async lettre). Supports Gmail, Outlook, custom servers.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage, ThreadType};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// Email channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub imap_host: String,
    #[serde(default = "default_imap_port")]
    pub imap_port: u16,
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default = "default_mailbox")]
    pub mailbox: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_true")]
    pub unread_only: bool,
    #[serde(default = "default_true")]
    pub mark_as_read: bool,
    #[serde(default = "default_true")]
    pub smtp_enabled: bool,
}

fn default_imap_port() -> u16 { 993 }
fn default_smtp_port() -> u16 { 587 }
fn default_mailbox() -> String { "INBOX".into() }
fn default_poll_interval() -> u64 { 30 }
fn default_true() -> bool { true }

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            imap_host: "imap.gmail.com".into(),
            imap_port: 993,
            smtp_host: "smtp.gmail.com".into(),
            smtp_port: 587,
            email: String::new(),
            password: String::new(),
            display_name: None,
            mailbox: "INBOX".into(),
            poll_interval_secs: 30,
            unread_only: true,
            mark_as_read: true,
            smtp_enabled: true,
        }
    }
}

/// Parsed email data.
#[derive(Debug, Clone)]
pub struct ParsedEmail {
    pub uid: u32,
    pub from: String,
    pub from_name: Option<String>,
    pub subject: String,
    pub body_text: String,
    pub message_id: Option<String>,
}

/// Email channel — IMAP reading + SMTP sending.
pub struct EmailChannel {
    config: EmailConfig,
    connected: bool,
    last_seen_uid: Arc<Mutex<u32>>,
}

impl EmailChannel {
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
            connected: false,
            last_seen_uid: Arc::new(Mutex::new(0)),
        }
    }

    /// Fetch unread emails (sync IMAP inside spawn_blocking).
    pub async fn fetch_unread(&self) -> Result<Vec<ParsedEmail>> {
        let host = self.config.imap_host.clone();
        let port = self.config.imap_port;
        let email = self.config.email.clone();
        let password = self.config.password.clone();
        let mailbox = self.config.mailbox.clone();
        let unread_only = self.config.unread_only;
        let mark_as_read = self.config.mark_as_read;
        let last_seen_uid = self.last_seen_uid.clone();

        tokio::task::spawn_blocking(move || {
            imap_fetch_sync(
                &host, port, &email, &password, &mailbox,
                unread_only, mark_as_read, &last_seen_uid,
            )
        })
        .await
        .map_err(|e| BizClawError::Channel(format!("IMAP task panicked: {e}")))?
    }

    /// Send email via SMTP (async).
    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        in_reply_to: Option<&str>,
    ) -> Result<()> {
        use lettre::{
            message::header::ContentType,
            transport::smtp::authentication::Credentials,
            AsyncSmtpTransport, AsyncTransport, Message as LettreMessage,
            message::Mailbox,
        };

        let from_name = self.config.display_name.as_deref().unwrap_or("BizClaw AI");
        let from_mailbox: Mailbox = format!("{from_name} <{}>", self.config.email)
            .parse()
            .map_err(|e| BizClawError::Channel(format!("Invalid from: {e}")))?;

        let to_mailbox: Mailbox = to.parse()
            .map_err(|e| BizClawError::Channel(format!("Invalid to: {e}")))?;

        let mut builder = LettreMessage::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN);

        if let Some(reply_id) = in_reply_to {
            builder = builder.in_reply_to(reply_id.to_string());
        }

        let email = builder
            .body(body.to_string())
            .map_err(|e| BizClawError::Channel(format!("Build email: {e}")))?;

        let creds = Credentials::new(
            self.config.email.clone(),
            self.config.password.clone(),
        );

        let mailer = AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(
            &self.config.smtp_host,
        )
        .map_err(|e| BizClawError::Channel(format!("SMTP relay: {e}")))?
        .port(self.config.smtp_port)
        .credentials(creds)
        .build();

        mailer.send(email).await
            .map_err(|e| BizClawError::Channel(format!("SMTP send: {e}")))?;

        tracing::info!("📤 Email sent to: {to}");
        Ok(())
    }

    /// Start IMAP polling loop — returns a stream of IncomingMessages.
    pub fn start_polling(self) -> EmailPollingStream {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let config = self.config.clone();
        let last_seen = self.last_seen_uid.clone();

        tokio::spawn(async move {
            let ch = EmailChannel {
                config,
                connected: true,
                last_seen_uid: last_seen,
            };
            loop {
                match ch.fetch_unread().await {
                    Ok(emails) => {
                        for em in emails {
                            let incoming = IncomingMessage {
                                channel: "email".into(),
                                thread_id: em.from.clone(),
                                sender_id: em.from.clone(),
                                sender_name: em.from_name.clone(),
                                content: format!("📧 Subject: {}\n\n{}", em.subject, em.body_text),
                                thread_type: ThreadType::Direct,
                                timestamp: chrono::Utc::now(),
                                reply_to: em.message_id,
                            };
                            if tx.send(incoming).is_err() { return; }
                        }
                    }
                    Err(e) => tracing::error!("IMAP poll: {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(ch.config.poll_interval_secs)).await;
            }
        });

        EmailPollingStream { rx }
    }
}

/// Stream of incoming email messages.
pub struct EmailPollingStream {
    rx: tokio::sync::mpsc::UnboundedReceiver<IncomingMessage>,
}

impl Stream for EmailPollingStream {
    type Item = IncomingMessage;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}
impl Unpin for EmailPollingStream {}

#[async_trait]
impl Channel for EmailChannel {
    fn name(&self) -> &str { "email" }

    async fn connect(&mut self) -> Result<()> {
        let host = self.config.imap_host.clone();
        let port = self.config.imap_port;
        let email = self.config.email.clone();
        let password = self.config.password.clone();

        tokio::task::spawn_blocking(move || {
            let tls = native_tls::TlsConnector::builder().build()
                .map_err(|e| BizClawError::Channel(format!("TLS: {e}")))?;
            let client = imap::connect(
                (host.as_str(), port), &host, &tls,
            ).map_err(|e| BizClawError::Channel(format!("IMAP connect: {e}")))?;
            let mut session = client.login(&email, &password)
                .map_err(|e| BizClawError::AuthFailed(format!("IMAP auth: {}", e.0)))?;
            session.logout().ok();
            Ok::<(), BizClawError>(())
        })
        .await
        .map_err(|e| BizClawError::Channel(format!("Task: {e}")))??;

        self.connected = true;
        tracing::info!("📧 Email connected: {}", self.config.email);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool { self.connected }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let subject = message.reply_to.as_deref()
            .map(|r| format!("Re: {r}"))
            .unwrap_or_else(|| "From BizClaw AI".into());
        self.send_email(&message.thread_id, &subject, &message.content, message.reply_to.as_deref()).await
    }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(futures::stream::pending()))
    }
}

/// Synchronous IMAP fetch — called inside spawn_blocking.
fn imap_fetch_sync(
    host: &str,
    port: u16,
    email: &str,
    password: &str,
    mailbox: &str,
    unread_only: bool,
    mark_as_read: bool,
    last_seen_uid: &Arc<Mutex<u32>>,
) -> Result<Vec<ParsedEmail>> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| BizClawError::Channel(format!("TLS: {e}")))?;

    let client = imap::connect((host, port), host, &tls)
        .map_err(|e| BizClawError::Channel(format!("IMAP connect: {e}")))?;

    let mut session = client.login(email, password)
        .map_err(|e| BizClawError::Channel(format!("IMAP login: {}", e.0)))?;

    session.select(mailbox)
        .map_err(|e| BizClawError::Channel(format!("Select: {e}")))?;

    let search = if unread_only { "UNSEEN" } else { "ALL" };
    let uids = session.uid_search(search)
        .map_err(|e| BizClawError::Channel(format!("Search: {e}")))?;

    let last = *last_seen_uid.lock().unwrap();
    let new_uids: Vec<u32> = uids.into_iter().filter(|&u| u > last).collect();

    if new_uids.is_empty() {
        session.logout().ok();
        return Ok(vec![]);
    }

    let uid_set = new_uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
    let messages = session.uid_fetch(&uid_set, "(UID RFC822)")
        .map_err(|e| BizClawError::Channel(format!("Fetch: {e}")))?;

    let mut emails = Vec::new();
    let mut max_uid = last;

    for msg in messages.iter() {
        let uid = msg.uid.unwrap_or(0);
        if uid > max_uid { max_uid = uid; }
        if let Some(body) = msg.body() {
            if let Some(parsed) = parse_email_bytes(body, uid) {
                emails.push(parsed);
            }
        }
    }

    if mark_as_read && !new_uids.is_empty() {
        session.uid_store(&uid_set, "+FLAGS (\\Seen)").ok();
    }

    *last_seen_uid.lock().unwrap() = max_uid;
    session.logout().ok();
    tracing::info!("📧 Fetched {} email(s)", emails.len());
    Ok(emails)
}

/// Parse raw email bytes.
fn parse_email_bytes(raw: &[u8], uid: u32) -> Option<ParsedEmail> {
    use mail_parser::MessageParser;
    let parsed = MessageParser::default().parse(raw)?;

    let from = parsed.from()
        .and_then(|a| a.first())
        .map(|a| a.address().unwrap_or_default().to_string())
        .unwrap_or_default();

    let from_name = parsed.from()
        .and_then(|a| a.first())
        .and_then(|a| a.name())
        .map(String::from);

    let subject = parsed.subject().unwrap_or("(no subject)").to_string();

    let body_text = parsed.body_text(0)
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            parsed.body_html(0)
                .map(|h| strip_html(&h))
                .unwrap_or_default()
        });

    let message_id = parsed.message_id().map(String::from);

    Some(ParsedEmail {
        uid, from, from_name, subject,
        body_text: body_text.chars().take(4000).collect(),
        message_id,
    })
}

fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}
