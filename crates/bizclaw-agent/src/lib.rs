//! # BizClaw Agent
//! The core agent engine — orchestrates providers, channels, memory, and tools.

pub mod engine;
pub mod context;

use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Provider;
use bizclaw_core::traits::SecurityPolicy;
use bizclaw_core::traits::memory::MemoryBackend;
use bizclaw_core::traits::provider::GenerateParams;
use bizclaw_core::types::{Message, OutgoingMessage};

/// Prompt cache — caches serialized system prompt + tool definitions to avoid
/// re-serializing on every request. Speeds up repeated calls significantly.
struct PromptCache {
    /// Hash of system prompt for change detection
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
    fn tool_defs(&mut self, tools: &bizclaw_tools::ToolRegistry) -> &[bizclaw_core::types::ToolDefinition] {
        if self.last_refresh.elapsed() > std::time::Duration::from_secs(300) {
            self.cached_tool_defs = tools.list();
            self.last_refresh = std::time::Instant::now();
        }
        &self.cached_tool_defs
    }
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
}

impl Agent {
    /// Create a new agent from configuration (sync, no MCP).
    pub fn new(config: BizClawConfig) -> Result<Self> {
        let provider = bizclaw_providers::create_provider(&config)?;
        let memory = bizclaw_memory::create_memory(&config.memory)?;
        let tools = bizclaw_tools::ToolRegistry::with_defaults();
        let security = bizclaw_security::DefaultSecurityPolicy::new(config.autonomy.clone());

        let prompt_cache = PromptCache::new(&config.identity.system_prompt, &tools);

        let mut conversation = vec![];
        conversation.push(Message::system(&config.identity.system_prompt));

        Ok(Self {
            config,
            provider,
            memory,
            tools,
            security,
            conversation,
            prompt_cache,
        })
    }

    /// Create a new agent with MCP server support (async).
    /// Connects to all configured MCP servers and registers their tools.
    pub async fn new_with_mcp(config: BizClawConfig) -> Result<Self> {
        let provider = bizclaw_providers::create_provider(&config)?;
        let memory = bizclaw_memory::create_memory(&config.memory)?;
        let mut tools = bizclaw_tools::ToolRegistry::with_defaults();
        let security = bizclaw_security::DefaultSecurityPolicy::new(config.autonomy.clone());

        // Connect MCP servers and register their tools
        if !config.mcp_servers.is_empty() {
            tracing::info!("🔗 Connecting {} MCP server(s)...", config.mcp_servers.len());
            let mcp_configs: Vec<bizclaw_mcp::McpServerConfig> = config.mcp_servers.iter().map(|e| {
                bizclaw_mcp::McpServerConfig {
                    name: e.name.clone(),
                    command: e.command.clone(),
                    args: e.args.clone(),
                    env: e.env.clone(),
                    enabled: e.enabled,
                }
            }).collect();

            let results = bizclaw_mcp::bridge::connect_mcp_servers(&mcp_configs).await;
            let mut total_mcp_tools = 0;
            for (_client, bridges) in results {
                total_mcp_tools += bridges.len();
                tools.register_many(bridges);
            }
            if total_mcp_tools > 0 {
                tracing::info!("✅ {} MCP tool(s) registered", total_mcp_tools);
            }
        }

        let prompt_cache = PromptCache::new(&config.identity.system_prompt, &tools);

        let mut conversation = vec![];
        conversation.push(Message::system(&config.identity.system_prompt));

        Ok(Self {
            config,
            provider,
            memory,
            tools,
            security,
            conversation,
            prompt_cache,
        })
    }

    /// Process a user message and generate a response.
    /// Features: memory retrieval (RAG), multi-round tool calling (max 3 rounds).
    pub async fn process(&mut self, user_message: &str) -> Result<String> {
        // ═══════════════════════════════════════
        // Phase 1: Memory Retrieval (RAG-style)
        // ═══════════════════════════════════════
        // Search past conversations for relevant context
        let memory_context = self.retrieve_memory(user_message).await;
        if let Some(ref ctx) = memory_context {
            // Inject memory as a context message before the user message
            self.conversation.push(Message::system(&format!(
                "[Relevant past conversations]\n{ctx}\n[End of past conversations]"
            )));
        }

        // Add user message to conversation
        self.conversation.push(Message::user(user_message));

        // Trim conversation to prevent context overflow (keep system + last 40 messages)
        if self.conversation.len() > 41 {
            let system = self.conversation[0].clone();
            let keep = self.conversation.len() - 40;
            let tail: Vec<_> = self.conversation.drain(keep..).collect();
            self.conversation.clear();
            self.conversation.push(system);
            self.conversation.extend(tail);
        }

        // Get cached tool definitions (avoids re-serialization)
        let tool_defs = self.prompt_cache.tool_defs(&self.tools).to_vec();

        // Create generation params
        let params = GenerateParams {
            model: self.config.default_model.clone(),
            temperature: self.config.default_temperature,
            max_tokens: 4096,
            top_p: 0.9,
            stop: vec![],
        };

        // ═══════════════════════════════════════
        // Phase 2: Multi-round Tool Calling Loop
        // ═══════════════════════════════════════
        // Allow up to MAX_TOOL_ROUNDS rounds of tool calls
        const MAX_TOOL_ROUNDS: usize = 3;
        let mut final_content = String::new();

        for round in 0..=MAX_TOOL_ROUNDS {
            // Call the provider (with tools on rounds 0..MAX, without on final)
            let current_tools = if round < MAX_TOOL_ROUNDS { &tool_defs } else { &vec![] };
            let response = self.provider.chat(&self.conversation, current_tools, &params).await?;

            // No tool calls → this is the final text response
            if response.tool_calls.is_empty() {
                final_content = response.content.unwrap_or_else(|| "I'm not sure how to respond.".into());
                self.conversation.push(Message::assistant(&final_content));
                break;
            }

            // Has tool calls → execute them
            tracing::info!("Tool round {}/{}: {} tool call(s)",
                round + 1, MAX_TOOL_ROUNDS, response.tool_calls.len());

            let mut tool_results = Vec::new();

            for tc in &response.tool_calls {
                tracing::info!("  → {} ({})", tc.function.name,
                    &tc.function.arguments[..tc.function.arguments.len().min(100)]);

                // Security check for shell commands
                if tc.function.name == "shell" {
                    if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                        if let Some(cmd) = args["command"].as_str() {
                            if !self.security.check_command(cmd).await? {
                                tool_results.push(Message::tool(
                                    format!("Permission denied: command '{}' not allowed", cmd),
                                    &tc.id,
                                ));
                                continue;
                            }
                        }
                    }
                }

                // Execute tool
                if let Some(tool) = self.tools.get(&tc.function.name) {
                    match tool.execute(&tc.function.arguments).await {
                        Ok(result) => {
                            // Truncate large outputs to avoid context overflow
                            let output = if result.output.len() > 4000 {
                                format!("{}...\n[truncated, {} total chars]",
                                    &result.output[..4000], result.output.len())
                            } else {
                                result.output
                            };
                            tool_results.push(Message::tool(&output, &tc.id));
                        }
                        Err(e) => {
                            tool_results.push(Message::tool(
                                format!("Tool error: {e}"), &tc.id,
                            ));
                        }
                    }
                } else {
                    tool_results.push(Message::tool(
                        format!("Tool not found: {}", tc.function.name), &tc.id,
                    ));
                }
            }

            // Add assistant message with tool calls to conversation
            self.conversation.push(Message {
                role: bizclaw_core::types::Role::Assistant,
                content: response.content.clone().unwrap_or_default(),
                name: None,
                tool_call_id: None,
                tool_calls: Some(response.tool_calls.clone()),
            });

            // Add tool results to conversation
            for tr in tool_results {
                self.conversation.push(tr);
            }

            // Loop continues → provider will see tool results and decide next action
        }

        // If we exhausted all rounds without a final text response
        if final_content.is_empty() {
            final_content = "I executed the requested tools.".into();
            self.conversation.push(Message::assistant(&final_content));
        }

        // ═══════════════════════════════════════
        // Phase 3: Save to Memory
        // ═══════════════════════════════════════
        self.save_memory(user_message, &final_content).await;

        Ok(final_content)
    }

    /// Retrieve relevant past conversations from memory.
    /// Extracts keywords from the user message and searches SQLite.
    async fn retrieve_memory(&self, user_message: &str) -> Option<String> {
        if !self.config.memory.auto_save {
            return None;
        }

        // Extract meaningful keywords (skip common words)
        let stop_words: std::collections::HashSet<&str> = [
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "shall", "can", "need", "dare", "ought",
            "i", "me", "my", "you", "your", "he", "she", "it", "we", "they",
            "this", "that", "these", "those", "what", "which", "who", "how",
            "and", "but", "or", "not", "no", "of", "in", "on", "at", "to",
            "for", "with", "from", "by", "as", "if", "then", "so", "than",
            "tôi", "bạn", "là", "có", "và", "của", "với", "cho", "để",
            "không", "được", "này", "đó", "một", "các", "những",
        ].iter().copied().collect();

        let keywords: Vec<&str> = user_message
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 2 && !stop_words.contains(&w.to_lowercase().as_str()))
            .take(5) // Max 5 keywords
            .collect();

        if keywords.is_empty() {
            return None;
        }

        // Search memory for each keyword, collect unique results
        let mut seen = std::collections::HashSet::new();
        let mut relevant = Vec::new();

        for keyword in &keywords {
            match self.memory.search(keyword, 3).await {
                Ok(results) => {
                    for r in results {
                        if seen.insert(r.entry.id.clone()) {
                            relevant.push(r.entry.content.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Memory search for '{}' failed: {e}", keyword);
                }
            }
        }

        if relevant.is_empty() {
            return None;
        }

        // Limit total context to ~2000 chars
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

        tracing::debug!("Memory retrieval: {} keywords → {} results, {} chars",
            keywords.len(), relevant.len(), total_len);

        Some(context)
    }

    /// Save interaction to memory (internal).
    async fn save_memory(&self, user_msg: &str, assistant_msg: &str) {
        if self.config.memory.auto_save {
            let entry = bizclaw_core::traits::memory::MemoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                content: format!("User: {user_msg}\nAssistant: {assistant_msg}"),
                metadata: serde_json::json!({}),
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
    /// Used by gateway WS handler when streaming bypasses the Agent engine.
    pub async fn save_memory_public(&self, user_msg: &str, assistant_msg: &str) {
        self.save_memory(user_msg, assistant_msg).await;
    }

    /// Process incoming message and create an outgoing response.
    pub async fn handle_incoming(&mut self, msg: &bizclaw_core::types::IncomingMessage) -> Result<OutgoingMessage> {
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
}
