//! # BizClaw Agent
//! The core agent engine — orchestrates providers, channels, memory, and tools.
//!
//! ## Features (BizClaw agent features):
//! - **Multi-round tool calling**: Up to 3 rounds of tool → LLM → tool loops
//! - **Memory retrieval (RAG)**: FTS5-powered search of past conversations
//! - **Knowledge base integration**: Auto-search uploaded documents for context
//! - **Auto-compaction**: Summarizes long conversations to prevent context overflow
//! - **Session management**: Thread isolation via session_id
//! - **Context tracking**: Monitor conversation length and estimate token usage

pub mod context;
pub mod engine;
pub mod orchestrator;
pub mod proactive;

use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Provider;
use bizclaw_core::traits::SecurityPolicy;
use bizclaw_core::traits::memory::MemoryBackend;
use bizclaw_core::traits::provider::GenerateParams;
use bizclaw_core::types::{Message, OutgoingMessage};

/// Prompt cache — caches serialized system prompt + tool definitions to avoid
/// re-serializing on every request.
struct PromptCache {
    /// Hash of system prompt for change detection
    #[allow(dead_code)]
    system_prompt_hash: u64,
    /// Pre-serialized tool definitions ready for provider API
    cached_tool_defs: Vec<bizclaw_core::types::ToolDefinition>,
    /// Timestamp of last cache refresh
    last_refresh: std::time::Instant,
}

impl PromptCache {
    fn new(system_prompt: &str, tools: &bizclaw_tools::ToolRegistry) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        system_prompt.hash(&mut hasher);
        let hash = hasher.finish();

        Self {
            system_prompt_hash: hash,
            cached_tool_defs: tools.list(),
            last_refresh: std::time::Instant::now(),
        }
    }

    /// Get cached tool definitions (refresh every 5 minutes).
    fn tool_defs(
        &mut self,
        tools: &bizclaw_tools::ToolRegistry,
    ) -> &[bizclaw_core::types::ToolDefinition] {
        if self.last_refresh.elapsed() > std::time::Duration::from_secs(300) {
            self.cached_tool_defs = tools.list();
            self.last_refresh = std::time::Instant::now();
        }
        &self.cached_tool_defs
    }
}

/// Context statistics for monitoring.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextStats {
    /// Number of messages in conversation
    pub message_count: usize,
    /// Estimated token count (rough: 4 chars ≈ 1 token)
    pub estimated_tokens: usize,
    /// Context utilization percentage (based on max_context)
    pub utilization_pct: f32,
    /// Max context window size
    pub max_context: usize,
    /// Number of tool rounds executed in last request
    pub last_tool_rounds: usize,
    /// Whether auto-compaction was triggered
    pub compacted: bool,
    /// Current session ID
    pub session_id: String,
}

/// The BizClaw agent — processes messages using LLM providers and tools.
pub struct Agent {
    config: BizClawConfig,
    provider: Box<dyn Provider>,
    memory: Box<dyn MemoryBackend>,
    tools: bizclaw_tools::ToolRegistry,
    security: bizclaw_security::DefaultSecurityPolicy,
    conversation: Vec<Message>,
    prompt_cache: PromptCache,
    /// Current session ID for memory isolation
    session_id: String,
    /// Knowledge base for RAG (optional, shared with gateway)
    knowledge:
        Option<std::sync::Arc<tokio::sync::Mutex<Option<bizclaw_knowledge::KnowledgeStore>>>>,
    /// Context statistics from last process() call
    last_stats: ContextStats,
    /// 3-Tier Memory: daily log manager for persisting compaction summaries
    daily_log: bizclaw_memory::brain::DailyLogManager,
}

impl Agent {
    /// Create a new agent from configuration (sync, no MCP).
    pub fn new(config: BizClawConfig) -> Result<Self> {
        let provider = bizclaw_providers::create_provider(&config)?;
        let memory = bizclaw_memory::create_memory(&config.memory)?;
        let tools = bizclaw_tools::ToolRegistry::with_defaults();
        let security = bizclaw_security::DefaultSecurityPolicy::new(config.autonomy.clone());

        // 3-Tier Memory: assemble brain context from workspace files
        let brain_ws = bizclaw_memory::brain::BrainWorkspace::default();
        let _ = brain_ws.initialize(); // seed default files if missing
        let brain_context = brain_ws.assemble_brain();
        let daily_log = bizclaw_memory::brain::DailyLogManager::default();

        // Build system prompt: user config + brain workspace
        let system_prompt = if brain_context.trim().is_empty() {
            config.identity.system_prompt.clone()
        } else {
            format!("{}\n\n{}", config.identity.system_prompt, brain_context)
        };

        let prompt_cache = PromptCache::new(&system_prompt, &tools);

        let conversation = vec![Message::system(&system_prompt)];

        Ok(Self {
            config,
            provider,
            memory,
            tools,
            security,
            conversation,
            prompt_cache,
            session_id: "default".to_string(),
            knowledge: None,
            last_stats: ContextStats {
                message_count: 1,
                estimated_tokens: 0,
                utilization_pct: 0.0,
                max_context: 128000,
                last_tool_rounds: 0,
                compacted: false,
                session_id: "default".to_string(),
            },
            daily_log,
        })
    }

    /// Create a new agent with MCP server support (async).
    pub async fn new_with_mcp(config: BizClawConfig) -> Result<Self> {
        // CRITICAL: create_provider is sync and can block (e.g., brain GGUF loading).
        // Run it on a blocking thread so it doesn't stall the tokio runtime.
        let config_clone = config.clone();
        let provider = tokio::task::spawn_blocking(move || {
            bizclaw_providers::create_provider(&config_clone)
        }).await.map_err(|e| bizclaw_core::error::BizClawError::Other(format!("spawn: {e}")))??;
        let memory = bizclaw_memory::create_memory(&config.memory)?;
        let mut tools = bizclaw_tools::ToolRegistry::with_defaults();
        let security = bizclaw_security::DefaultSecurityPolicy::new(config.autonomy.clone());

        // Connect MCP servers and register their tools
        if !config.mcp_servers.is_empty() {
            tracing::info!(
                "🔗 Connecting {} MCP server(s)...",
                config.mcp_servers.len()
            );
            let mcp_configs: Vec<bizclaw_mcp::McpServerConfig> = config
                .mcp_servers
                .iter()
                .map(|e| bizclaw_mcp::McpServerConfig {
                    name: e.name.clone(),
                    command: e.command.clone(),
                    args: e.args.clone(),
                    env: e.env.clone(),
                    enabled: e.enabled,
                })
                .collect();

            let results = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                bizclaw_mcp::bridge::connect_mcp_servers(&mcp_configs),
            ).await;
            let mut total_mcp_tools = 0;
            match results {
                Ok(connections) => {
                    for (_client, bridges) in connections {
                        total_mcp_tools += bridges.len();
                        tools.register_many(bridges);
                    }
                }
                Err(_) => {
                    tracing::warn!("⚠️ MCP server connection timed out (10s), skipping");
                }
            }
            if total_mcp_tools > 0 {
                tracing::info!("✅ {} MCP tool(s) registered", total_mcp_tools);
            }
        }

        // 3-Tier Memory: assemble brain context from workspace files
        let brain_ws = bizclaw_memory::brain::BrainWorkspace::default();
        let _ = brain_ws.initialize();
        let brain_context = brain_ws.assemble_brain();
        let daily_log = bizclaw_memory::brain::DailyLogManager::default();

        let system_prompt = if brain_context.trim().is_empty() {
            config.identity.system_prompt.clone()
        } else {
            format!("{}\n\n{}", config.identity.system_prompt, brain_context)
        };

        let prompt_cache = PromptCache::new(&system_prompt, &tools);

        let conversation = vec![Message::system(&system_prompt)];

        Ok(Self {
            config,
            provider,
            memory,
            tools,
            security,
            conversation,
            prompt_cache,
            session_id: "default".to_string(),
            knowledge: None,
            daily_log,
            last_stats: ContextStats {
                message_count: 1,
                estimated_tokens: 0,
                utilization_pct: 0.0,
                max_context: 128000,
                last_tool_rounds: 0,
                compacted: false,
                session_id: "default".to_string(),
            },
        })
    }

    /// Attach a knowledge base for RAG-enhanced responses.
    pub fn set_knowledge(
        &mut self,
        kb: std::sync::Arc<tokio::sync::Mutex<Option<bizclaw_knowledge::KnowledgeStore>>>,
    ) {
        self.knowledge = Some(kb);
    }

    /// Set the current session ID for memory isolation.
    pub fn set_session(&mut self, session_id: &str) {
        self.session_id = session_id.to_string();
        self.last_stats.session_id = session_id.to_string();
    }

    /// Get current session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Process a user message and generate a response.
    ///
    /// Uses Think-Act-Observe loop (inspired by [GoClaw](https://github.com/nextlevelbuilder/goclaw))
    /// with Quality Gate evaluation (inspired by [OpenFang](https://github.com/RightNow-AI/openfang)).
    pub async fn process(&mut self, user_message: &str) -> Result<String> {
        let mut compacted = false;
        let estimated_tokens = self.estimate_tokens();
        let max_context = self.config.brain.context_length as usize;
        let utilization = if max_context > 0 { estimated_tokens as f32 / max_context as f32 } else { 0.0 };

        if utilization > 0.70 && self.conversation.len() > 10 {
            tracing::info!("📦 Auto-compaction triggered ({}% used)", (utilization * 100.0) as u32);
            self.compact_conversation().await;
            compacted = true;
        }

        // Knowledge RAG
        if let Some(kb_ctx) = self.search_knowledge(user_message).await {
            self.conversation.push(Message::system(format!(
                "[Knowledge Base]\n{kb_ctx}\n[End knowledge]"
            )));
        }

        // Memory retrieval
        if let Some(mem_ctx) = self.retrieve_memory(user_message).await {
            self.conversation.push(Message::system(format!(
                "[Past conversations]\n{mem_ctx}\n[End past]"
            )));
        }

        self.conversation.push(Message::user(user_message));

        // Trim conversation
        if self.conversation.len() > 41 {
            let system = self.conversation[0].clone();
            let keep = self.conversation.len() - 40;
            let tail: Vec<_> = self.conversation.drain(keep..).collect();
            self.conversation.clear();
            self.conversation.push(system);
            self.conversation.extend(tail);
        }

        let tool_defs = self.prompt_cache.tool_defs(&self.tools).to_vec();
        let params = GenerateParams {
            model: self.config.default_model.clone(),
            temperature: self.config.default_temperature,
            max_tokens: self.config.brain.max_tokens,
            top_p: 0.9,
            stop: vec![],
        };

        // Think-Act-Observe Loop
        const MAX_ROUNDS: usize = 5;
        let mut final_content = String::new();
        let mut tool_rounds = 0;

        for round in 0..=MAX_ROUNDS {
            let tools = if round < MAX_ROUNDS { &tool_defs } else { &vec![] };
            tracing::debug!("🧠 Think round {}/{}", round + 1, MAX_ROUNDS);

            let resp = self.provider.chat(&self.conversation, tools, &params).await?;

            if resp.tool_calls.is_empty() {
                final_content = resp.content.unwrap_or_else(|| "I'm not sure how to respond.".into());
                self.conversation.push(Message::assistant(&final_content));
                break;
            }

            // ACT
            tool_rounds = round + 1;
            tracing::info!("⚡ Act round {}: {} tool(s)", tool_rounds, resp.tool_calls.len());

            let mut results = Vec::new();
            for tc in &resp.tool_calls {
                tracing::info!("  → {}", tc.function.name);
                if tc.function.name == "shell"
                    && let Ok(args) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                    && let Some(cmd) = args["command"].as_str()
                    && !self.security.check_command(cmd).await?
                {
                    results.push(Message::tool(format!("Permission denied: '{cmd}'"), &tc.id));
                    continue;
                }
                if let Some(tool) = self.tools.get(&tc.function.name) {
                    match tool.execute(&tc.function.arguments).await {
                        Ok(r) => {
                            let out = if r.output.len() > 4000 {
                                format!("{}...[truncated]", &r.output[..4000])
                            } else { r.output };
                            results.push(Message::tool(&out, &tc.id));
                        }
                        Err(e) => results.push(Message::tool(format!("Error: {e}"), &tc.id)),
                    }
                } else {
                    results.push(Message::tool(format!("Not found: {}", tc.function.name), &tc.id));
                }
            }

            // OBSERVE
            self.conversation.push(Message {
                role: bizclaw_core::types::Role::Assistant,
                content: resp.content.clone().unwrap_or_default(),
                name: None, tool_call_id: None,
                tool_calls: Some(resp.tool_calls.clone()),
            });
            for r in results { self.conversation.push(r); }
            tracing::debug!("🔍 Observe — looping to Think");
        }

        if final_content.is_empty() {
            final_content = "I executed the requested tools.".into();
            self.conversation.push(Message::assistant(&final_content));
        }

        // Quality Gate
        if let Some(ref gate) = self.config.quality_gate
            && !gate.evaluator_prompt.is_empty() {
                let max_rev = gate.max_revisions.unwrap_or(2) as usize;
                for rev in 0..max_rev {
                    let ep = format!("{}\n\nUSER: {}\nRESPONSE: {}\n\nReply APPROVED or REVISION_NEEDED: <feedback>",
                        gate.evaluator_prompt, user_message, final_content);
                    let em = vec![Message::system("Quality evaluator."), Message::user(&ep)];
                    let epar = GenerateParams {
                        model: gate.evaluator_model.clone().unwrap_or(self.config.default_model.clone()),
                        temperature: 0.3, max_tokens: 500, top_p: 0.9, stop: vec![],
                    };
                    match self.provider.chat(&em, &[], &epar).await {
                        Ok(er) => {
                            let e = er.content.unwrap_or_default();
                            if e.contains("APPROVED") { tracing::info!("✅ QG passed"); break; }
                            if e.contains("REVISION_NEEDED") {
                                tracing::info!("🔄 Revision {}/{}", rev+1, max_rev);
                                let fb = e.split_once(':').map(|x| x.1).unwrap_or("Improve.");
                                self.conversation.push(Message::system(format!("[QG rev {}/{}] {}", rev+1, max_rev, fb.trim())));
                                if let Ok(rv) = self.provider.chat(&self.conversation, &[], &params).await
                                    && let Some(nc) = rv.content {
                                        final_content = nc;
                                        self.conversation.push(Message::assistant(&final_content));
                                    }
                            } else { break; }
                        }
                        Err(_) => break,
                    }
                }
            }

        // Save memory + update stats
        self.save_memory(user_message, &final_content).await;
        let new_tokens = self.estimate_tokens();
        self.last_stats = ContextStats {
            message_count: self.conversation.len(),
            estimated_tokens: new_tokens,
            utilization_pct: new_tokens as f32 / max_context as f32 * 100.0,
            max_context, last_tool_rounds: tool_rounds, compacted,
            session_id: self.session_id.clone(),
        };

        Ok(final_content)
    }


    /// Search the knowledge base for relevant context.
    async fn search_knowledge(&self, query: &str) -> Option<String> {
        let kb_arc = self.knowledge.as_ref()?;
        let kb_lock = kb_arc.lock().await;
        let kb = kb_lock.as_ref()?;

        let results = kb.search(query, 3);
        if results.is_empty() {
            return None;
        }

        let mut context = String::new();
        for (i, r) in results.iter().enumerate() {
            let entry = format!("{}. [{}] {}\n", i + 1, r.doc_name, r.content);
            if context.len() + entry.len() > 1500 {
                break;
            }
            context.push_str(&entry);
        }

        tracing::debug!(
            "Knowledge RAG: {} results, {} chars",
            results.len(),
            context.len()
        );
        Some(context)
    }

    /// Retrieve relevant past conversations from memory (FTS5-powered).
    async fn retrieve_memory(&self, user_message: &str) -> Option<String> {
        if !self.config.memory.auto_save {
            return None;
        }

        // Extract meaningful keywords (skip common words)
        let stop_words: std::collections::HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
            "shall", "can", "need", "dare", "ought", "i", "me", "my", "you", "your", "he", "she",
            "it", "we", "they", "this", "that", "these", "those", "what", "which", "who", "how",
            "and", "but", "or", "not", "no", "of", "in", "on", "at", "to", "for", "with", "from",
            "by", "as", "if", "then", "so", "than", "tôi", "bạn", "là", "có", "và", "của", "với",
            "cho", "để", "không", "được", "này", "đó", "một", "các", "những",
        ]
        .iter()
        .copied()
        .collect();

        let keywords: Vec<&str> = user_message
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 2 && !stop_words.contains(&w.to_lowercase().as_str()))
            .take(5)
            .collect();

        if keywords.is_empty() {
            return None;
        }

        // Search memory with combined keywords for better FTS5 results
        let combined_query = keywords.join(" ");
        let mut relevant = Vec::new();
        let mut seen = std::collections::HashSet::new();

        match self.memory.search(&combined_query, 5).await {
            Ok(results) => {
                for r in results {
                    if seen.insert(r.entry.id.clone()) {
                        relevant.push(r.entry.content.clone());
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Memory search failed: {e}");
            }
        }

        if relevant.is_empty() {
            return None;
        }

        let mut context = String::new();
        let mut total_len = 0;
        for (i, memory) in relevant.iter().take(5).enumerate() {
            let entry = format!("{}. {}\n", i + 1, memory);
            if total_len + entry.len() > 2000 {
                break;
            }
            context.push_str(&entry);
            total_len += entry.len();
        }

        tracing::debug!(
            "Memory RAG: {} results, {} chars",
            relevant.len(),
            total_len
        );
        Some(context)
    }

    /// Save interaction to memory with session ID.
    async fn save_memory(&self, user_msg: &str, assistant_msg: &str) {
        if self.config.memory.auto_save {
            let entry = bizclaw_core::traits::memory::MemoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                content: format!("User: {user_msg}\nAssistant: {assistant_msg}"),
                metadata: serde_json::json!({
                    "session_id": self.session_id,
                }),
                embedding: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            if let Err(e) = self.memory.save(entry).await {
                tracing::warn!("Failed to save memory: {e}");
            }
        }
    }

    /// Public wrapper to save streamed conversations to memory.
    pub async fn save_memory_public(&self, user_msg: &str, assistant_msg: &str) {
        self.save_memory(user_msg, assistant_msg).await;
    }

    /// Auto-compact conversation when context is too large.
    /// Keeps system prompt + summary of old messages + recent messages.
    async fn compact_conversation(&mut self) {
        if self.conversation.len() <= 10 {
            return;
        }

        let system = self.conversation[0].clone();

        // Summarize old messages (keep last 10)
        let old_count = self.conversation.len() - 10;
        let old_messages: Vec<_> = self.conversation[1..=old_count].to_vec();
        let recent: Vec<_> = self.conversation[old_count + 1..].to_vec();

        // Create a summary of old messages
        let mut summary_parts = Vec::new();
        for msg in &old_messages {
            let prefix = match msg.role {
                bizclaw_core::types::Role::User => "User",
                bizclaw_core::types::Role::Assistant => "AI",
                bizclaw_core::types::Role::System => continue, // skip system messages
                bizclaw_core::types::Role::Tool => "Tool",
            };
            // Take first 100 chars of each message
            let content = if msg.content.len() > 100 {
                format!("{}...", &msg.content[..100])
            } else {
                msg.content.clone()
            };
            summary_parts.push(format!("{prefix}: {content}"));
        }

        let summary = format!(
            "[Compacted: {} earlier messages]\n{}\n[End of compacted context]",
            old_count,
            summary_parts.join("\n")
        );

        // Rebuild conversation: system + summary + recent
        self.conversation.clear();
        self.conversation.push(system);
        self.conversation.push(Message::system(&summary));
        self.conversation.extend(recent);

        tracing::info!(
            "📦 Compacted {} → {} messages",
            old_count + 10,
            self.conversation.len()
        );

        // 3-Tier Memory: persist compaction summary to daily log
        if let Err(e) = self.daily_log.save_compaction(&summary) {
            tracing::warn!("Failed to save compaction to daily log: {e}");
        }
    }

    /// Estimate token count (rough heuristic: 1 token ≈ 4 chars for English, 2 chars for CJK).
    fn estimate_tokens(&self) -> usize {
        self.conversation
            .iter()
            .map(|m| {
                let chars = m.content.len();
                // Rough estimate: mix of English and Vietnamese
                chars / 3
            })
            .sum()
    }

    /// Process incoming message and create an outgoing response.
    pub async fn handle_incoming(
        &mut self,
        msg: &bizclaw_core::types::IncomingMessage,
    ) -> Result<OutgoingMessage> {
        let response = self.process(&msg.content).await?;
        Ok(OutgoingMessage {
            thread_id: msg.thread_id.clone(),
            content: response,
            thread_type: msg.thread_type.clone(),
            reply_to: None,
        })
    }

    /// Get provider name.
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Get model name.
    pub fn model_name(&self) -> &str {
        &self.config.default_model
    }

    /// Get system prompt.
    pub fn system_prompt(&self) -> &str {
        &self.config.identity.system_prompt
    }

    /// Update system prompt in-place (without re-creating agent).
    /// Updates both the config and the first message in conversation history.
    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.config.identity.system_prompt = prompt.to_string();
        // Also update the system message in conversation (always at index 0)
        if !self.conversation.is_empty() {
            // Rebuild with brain context same as new()
            let brain_ws = bizclaw_memory::brain::BrainWorkspace::default();
            let brain_context = brain_ws.assemble_brain();
            let full_prompt = if brain_context.trim().is_empty() {
                prompt.to_string()
            } else {
                format!("{}\n\n{}", prompt, brain_context)
            };
            self.conversation[0] = Message::system(&full_prompt);
        }
    }

    /// Get total tool count (native + MCP).
    pub fn tool_count(&self) -> usize {
        self.tools.list().len()
    }

    /// Get conversation history.
    pub fn conversation(&self) -> &[Message] {
        &self.conversation
    }

    /// Clear conversation history (keep system prompt).
    pub fn clear_conversation(&mut self) {
        self.conversation.truncate(1);
    }

    /// Get last context statistics.
    pub fn context_stats(&self) -> &ContextStats {
        &self.last_stats
    }
}
