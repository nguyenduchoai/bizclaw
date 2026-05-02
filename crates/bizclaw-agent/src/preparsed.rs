//! Pre-parsed local commands (bypass LLM).
//! Fast local commands that don't need LLM inference.

use serde::{Deserialize, Serialize};
use crate::context_summarizer::{apply_summarization, SummarizationConfig};
use bizclaw_core::types::message::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub args_required: usize,
    pub handler: &'static str,
    pub bypass_llm: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub command: LocalCommand,
    pub args: Option<String>,
    pub handler: &'static str,
}

pub struct LocalCommandParser {
    commands: Vec<(&'static str, LocalCommand)>,
}

impl Default for LocalCommandParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalCommandParser {
    pub fn new() -> Self {
        let commands = vec![
            ("/status", LocalCommand { name: "Status", description: "Show gateway status", args_required: 0, handler: "status_handler", bypass_llm: true }),
            ("/help", LocalCommand { name: "Help", description: "Show available commands", args_required: 0, handler: "help_handler", bypass_llm: true }),
            ("/clear", LocalCommand { name: "Clear", description: "Clear conversation history", args_required: 0, handler: "clear_handler", bypass_llm: true }),
            ("/compact", LocalCommand { name: "Compact", description: "Compact context window", args_required: 0, handler: "compact_handler", bypass_llm: true }),
            ("/model", LocalCommand { name: "Model", description: "Switch model", args_required: 1, handler: "model_handler", bypass_llm: true }),
            ("/tools", LocalCommand { name: "Tools", description: "List tools", args_required: 0, handler: "tools_handler", bypass_llm: true }),
            ("/health", LocalCommand { name: "Health", description: "Check system health", args_required: 0, handler: "health_handler", bypass_llm: true }),
        ];
        Self { commands }
    }

    pub fn parse(&self, input: &str) -> Option<ParsedCommand> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        let cmd_name = parts[0];
        
        self.commands.iter()
            .find(|(name, _)| *name == cmd_name)
            .map(|(_name, cmd)| ParsedCommand {
                command: cmd.clone(),
                args: parts.get(1).map(|s| s.to_string()),
                handler: cmd.handler,
            })
    }
    
    pub fn list_commands(&self) -> Vec<(&str, &LocalCommand)> {
        self.commands.iter().map(|(n, c)| (*n, c)).collect()
    }

    pub fn handle_compact(&self, messages: &mut Vec<Message>, session_id: &str, estimated_tokens: usize, max_tokens: usize) -> String {
        let config = SummarizationConfig::default();
        if let Some(result) = apply_summarization(messages, &config, estimated_tokens, max_tokens, session_id) {
            format!(
                "✅ Context compacted successfully!\n\nMessages compressed: {}\nTokens saved: ~{}\n\n{}",
                result.messages_compressed,
                result.tokens_saved,
                result.summary
            )
        } else {
            "⚠️ Compaction not needed yet. Context is under 80% threshold.".to_string()
        }
    }

    pub fn handle_status(&self, stats: &crate::ContextStats) -> String {
        format!(
            "📊 Gateway Status\n\n\
            Session: {}\n\
            Messages: {}\n\
            Tokens: ~{} / {}\n\
            Utilization: {:.1}%\n\
            Last tool rounds: {}\n\
            Compacted: {}",
            stats.session_id,
            stats.message_count,
            stats.estimated_tokens,
            stats.max_context,
            stats.utilization_pct * 100.0,
            stats.last_tool_rounds,
            if stats.compacted { "Yes" } else { "No" }
        )
    }

    pub fn handle_clear(&self) -> String {
        "🗑️ Conversation cleared.".to_string()
    }

    pub fn handle_tools(&self, available_tools: &[String]) -> String {
        if available_tools.is_empty() {
            "No tools available.".to_string()
        } else {
            let mut msg = "🔧 Available Tools:\n".to_string();
            for tool in available_tools {
                msg.push_str(&format!("  • {}\n", tool));
            }
            msg
        }
    }

    pub fn handle_health(&self) -> String {
        "✅ System healthy".to_string()
    }
}
