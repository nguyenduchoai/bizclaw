//! # BizClaw Agent
//! The core agent engine — orchestrates providers, channels, memory, and tools.

pub mod engine;
pub mod context;

use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Provider;
use bizclaw_core::traits::memory::MemoryBackend;
use bizclaw_core::traits::provider::GenerateParams;
use bizclaw_core::types::{Message, OutgoingMessage, ThreadType};

/// The BizClaw agent — processes messages using LLM providers and tools.
pub struct Agent {
    config: BizClawConfig,
    provider: Box<dyn Provider>,
    memory: Box<dyn MemoryBackend>,
    tools: bizclaw_tools::ToolRegistry,
    security: bizclaw_security::DefaultSecurityPolicy,
    conversation: Vec<Message>,
}

impl Agent {
    /// Create a new agent from configuration.
    pub fn new(config: BizClawConfig) -> Result<Self> {
        let provider = bizclaw_providers::create_provider(&config)?;
        let memory = bizclaw_memory::create_memory(&config.memory)?;
        let tools = bizclaw_tools::ToolRegistry::with_defaults();
        let security = bizclaw_security::DefaultSecurityPolicy::new(config.autonomy.clone());

        let mut conversation = vec![];
        conversation.push(Message::system(&config.identity.system_prompt));

        Ok(Self {
            config,
            provider,
            memory,
            tools,
            security,
            conversation,
        })
    }

    /// Process a user message and generate a response.
    pub async fn process(&mut self, user_message: &str) -> Result<String> {
        // Add user message to conversation
        self.conversation.push(Message::user(user_message));

        // Get tool definitions
        let tool_defs = self.tools.list();

        // Create generation params
        let params = GenerateParams {
            model: self.config.default_model.clone(),
            temperature: self.config.default_temperature,
            max_tokens: 4096,
            top_p: 0.9,
            stop: vec![],
        };

        // Call the provider
        let response = self.provider.chat(&self.conversation, &tool_defs, &params).await?;

        // Handle tool calls
        if !response.tool_calls.is_empty() {
            let mut tool_results = Vec::new();

            for tc in &response.tool_calls {
                tracing::info!("Tool call: {} with args: {}", tc.function.name, tc.function.arguments);

                // Security check
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
                            tool_results.push(Message::tool(&result.output, &tc.id));
                        }
                        Err(e) => {
                            tool_results.push(Message::tool(
                                format!("Tool error: {e}"),
                                &tc.id,
                            ));
                        }
                    }
                } else {
                    tool_results.push(Message::tool(
                        format!("Tool not found: {}", tc.function.name),
                        &tc.id,
                    ));
                }
            }

            // Add assistant message with tool calls
            self.conversation.push(Message {
                role: bizclaw_core::types::Role::Assistant,
                content: response.content.clone().unwrap_or_default(),
                name: None,
                tool_call_id: None,
                tool_calls: Some(response.tool_calls.clone()),
            });

            // Add tool results
            for tr in tool_results {
                self.conversation.push(tr);
            }

            // Get final response after tool execution
            let final_response = self.provider.chat(&self.conversation, &[], &params).await?;
            let content = final_response.content.unwrap_or_else(|| "I executed the tools.".into());
            self.conversation.push(Message::assistant(&content));

            // Save to memory
            self.save_memory(user_message, &content).await;

            return Ok(content);
        }

        // No tool calls — just text response
        let content = response.content.unwrap_or_else(|| "I'm not sure how to respond.".into());
        self.conversation.push(Message::assistant(&content));

        // Save to memory
        self.save_memory(user_message, &content).await;

        Ok(content)
    }

    /// Save interaction to memory.
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

    /// Get conversation history.
    pub fn conversation(&self) -> &[Message] {
        &self.conversation
    }

    /// Clear conversation history (keep system prompt).
    pub fn clear_conversation(&mut self) {
        self.conversation.truncate(1);
    }
}
