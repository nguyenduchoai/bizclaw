//! Zalo messaging - send/receive/forward/delete/stickers.

use serde::{Deserialize, Serialize};

/// Thread type in Zalo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ZaloThreadType {
    User,
    Group,
}

/// Send a text message.
pub async fn send_message(
    _content: &str,
    _thread_id: &str,
    _thread_type: ZaloThreadType,
) -> Result<String, String> {
    tracing::info!("Zalo: send_message (stub)");
    Ok("msg-id-stub".into())
}

/// Send a message with quote/reply.
pub async fn send_reply(
    _content: &str,
    _thread_id: &str,
    _thread_type: ZaloThreadType,
    _reply_to: &str,
) -> Result<String, String> {
    tracing::info!("Zalo: send_reply (stub)");
    Ok("msg-id-stub".into())
}

/// Forward a message.
pub async fn forward_message(
    _msg_id: &str,
    _to_thread_id: &str,
    _thread_type: ZaloThreadType,
) -> Result<(), String> {
    Ok(())
}

/// Delete a message.
pub async fn delete_message(_msg_id: &str) -> Result<(), String> {
    Ok(())
}

/// Undo (recall) a message.
pub async fn undo_message(_msg_id: &str) -> Result<(), String> {
    Ok(())
}
