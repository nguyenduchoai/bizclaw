//! # 8-Stage Agent Pipeline
//!
//! GoClaw-inspired pipeline cho standardized agent behavior:
//!
//! ## Stages:
//! 1. **Context** - Load context (brain files, memory, user info)
//! 2. **History** - Load conversation history
//! 3. **Prompt** - Build system prompt
//! 4. **Think** - LLM reasoning (with optional chain-of-thought)
//! 5. **Act** - Execute tools/actions
//! 6. **Observe** - Process tool results
//! 7. **Memory** - Save to memory
//! 8. **Summarize** - Generate session summary

use async_trait::async_trait;
use bizclaw_core::types::{Message, ProviderResponse, ToolCall};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineMode {
    Full,
    Task,
    Minimal,
    None,
}

impl PipelineMode {
    pub fn gates(&self) -> Vec<PipelineStage> {
        match self {
            PipelineMode::Full => vec![
                PipelineStage::Context,
                PipelineStage::History,
                PipelineStage::Prompt,
                PipelineStage::Think,
                PipelineStage::Act,
                PipelineStage::Observe,
                PipelineStage::Memory,
                PipelineStage::Summarize,
            ],
            PipelineMode::Task => vec![
                PipelineStage::Context,
                PipelineStage::Prompt,
                PipelineStage::Think,
                PipelineStage::Act,
                PipelineStage::Observe,
            ],
            PipelineMode::Minimal => vec![
                PipelineStage::Prompt,
                PipelineStage::Think,
                PipelineStage::Act,
            ],
            PipelineMode::None => vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    Context,
    History,
    Prompt,
    Think,
    Act,
    Observe,
    Memory,
    Summarize,
}

impl PipelineStage {
    pub fn as_str(&self) -> &str {
        match self {
            PipelineStage::Context => "context",
            PipelineStage::History => "history",
            PipelineStage::Prompt => "prompt",
            PipelineStage::Think => "think",
            PipelineStage::Act => "act",
            PipelineStage::Observe => "observe",
            PipelineStage::Memory => "memory",
            PipelineStage::Summarize => "summarize",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    pub user_id: String,
    pub session_id: String,
    pub mode: PipelineMode,
    pub brain_files: Vec<BrainFile>,
    pub conversation_history: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainFile {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub current_stage: PipelineStage,
    pub context: PipelineContext,
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
    pub memory_entries: Vec<MemoryEntry>,
    pub session_summary: Option<String>,
}

impl PipelineState {
    pub fn new(context: PipelineContext) -> Self {
        Self {
            current_stage: PipelineStage::Context,
            context,
            messages: Vec::new(),
            system_prompt: None,
            reasoning: None,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            memory_entries: Vec::new(),
            session_summary: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub result: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub content: String,
    pub importance: f32,
    pub tags: Vec<String>,
}

#[async_trait]
pub trait PipelineStageHandler: Send + Sync {
    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError>;
    fn stage(&self) -> PipelineStage;
}

pub struct ContextStage;
pub struct HistoryStage;
pub struct PromptStage;
pub struct ThinkStage;
pub struct ActStage;
pub struct ObserveStage;
pub struct MemoryStage;
pub struct SummarizeStage;

#[async_trait]
impl PipelineStageHandler for ContextStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::Context
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 1/8: Loading context...");

        let mut context_str = String::new();

        for brain_file in &state.context.brain_files {
            context_str.push_str(&format!(
                "\n[{}]\n{}\n",
                brain_file.name, brain_file.content
            ));
        }

        state.system_prompt = Some(context_str);

        tracing::info!(
            "✅ Context loaded ({} files)",
            state.context.brain_files.len()
        );
        Ok(())
    }
}

#[async_trait]
impl PipelineStageHandler for HistoryStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::History
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 2/8: Loading history...");

        let history_limit = 20;
        let recent = state
            .context
            .conversation_history
            .iter()
            .rev()
            .take(history_limit)
            .cloned()
            .collect::<Vec<_>>();

        for msg in recent.iter().rev() {
            state.messages.push(msg.clone());
        }

        tracing::info!("✅ History loaded ({} messages)", state.messages.len());
        Ok(())
    }
}

#[async_trait]
impl PipelineStageHandler for PromptStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::Prompt
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 3/8: Building prompt...");

        let mut prompt = state.system_prompt.clone().unwrap_or_default();

        prompt.push_str("\n\n[AVAILABLE TOOLS]\n");
        for tool in &state.context.tools {
            prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
        }

        prompt.push_str("\n[INSTRUCTIONS]\n");
        prompt.push_str("- Think step by step before responding\n");
        prompt.push_str("- Use tools when needed\n");
        prompt.push_str("- Always respond in Vietnamese unless asked otherwise\n");

        state.system_prompt = Some(prompt);

        tracing::info!("✅ Prompt built");
        Ok(())
    }
}

#[async_trait]
impl PipelineStageHandler for ThinkStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::Think
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 4/8: LLM reasoning...");

        Ok(())
    }
}

#[async_trait]
impl PipelineStageHandler for ActStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::Act
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 5/8: Executing actions...");

        Ok(())
    }
}

#[async_trait]
impl PipelineStageHandler for ObserveStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::Observe
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 6/8: Observing results...");

        Ok(())
    }
}

#[async_trait]
impl PipelineStageHandler for MemoryStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::Memory
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 7/8: Saving to memory...");

        let recent_msg = state.messages.last();
        if let Some(msg) = recent_msg {
            state.memory_entries.push(MemoryEntry {
                content: msg.content.clone(),
                importance: 0.5,
                tags: vec!["conversation".to_string()],
            });
        }

        tracing::info!("✅ Memory saved ({} entries)", state.memory_entries.len());
        Ok(())
    }
}

#[async_trait]
impl PipelineStageHandler for SummarizeStage {
    fn stage(&self) -> PipelineStage {
        PipelineStage::Summarize
    }

    async fn execute(&self, state: &mut PipelineState) -> Result<(), PipelineError> {
        tracing::info!("🔵 Stage 8/8: Generating summary...");

        if state.messages.len() > 10 {
            state.session_summary = Some(format!(
                "Session covered {} messages with {} tool calls",
                state.messages.len(),
                state.tool_calls.len()
            ));
        }

        tracing::info!("✅ Summary generated");
        Ok(())
    }
}

pub struct AgentPipeline {
    handlers: Vec<Box<dyn PipelineStageHandler>>,
    mode: PipelineMode,
}

impl AgentPipeline {
    pub fn new(mode: PipelineMode) -> Self {
        let handlers = if mode == PipelineMode::None {
            vec![]
        } else {
            vec![
                Box::new(ContextStage {}) as Box<dyn PipelineStageHandler>,
                Box::new(HistoryStage {}) as Box<dyn PipelineStageHandler>,
                Box::new(PromptStage {}) as Box<dyn PipelineStageHandler>,
                Box::new(ThinkStage {}) as Box<dyn PipelineStageHandler>,
                Box::new(ActStage {}) as Box<dyn PipelineStageHandler>,
                Box::new(ObserveStage {}) as Box<dyn PipelineStageHandler>,
                Box::new(MemoryStage {}) as Box<dyn PipelineStageHandler>,
                Box::new(SummarizeStage {}) as Box<dyn PipelineStageHandler>,
            ]
        };

        Self { handlers, mode }
    }

    pub fn mode(&self) -> &PipelineMode {
        &self.mode
    }

    pub async fn run(&self, mut state: PipelineState) -> Result<PipelineState, PipelineError> {
        let stages = self.mode.gates();
        let stage_count = stages.len();

        tracing::info!(
            "🚀 Starting pipeline (mode: {:?}, {} stages)",
            self.mode,
            stage_count
        );

        for (i, stage) in stages.iter().enumerate() {
            tracing::info!("📍 Stage {}/{}: {:?}", i + 1, stage_count, stage);
            state.current_stage = *stage;

            if let Some(handler) = self.handlers.iter().find(|h| h.stage() == *stage) {
                handler.execute(&mut state).await?;
            }
        }

        tracing::info!("✅ Pipeline completed");
        Ok(state)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Context error: {0}")]
    Context(String),

    #[error("History error: {0}")]
    History(String),

    #[error("Prompt error: {0}")]
    Prompt(String),

    #[error("Think error: {0}")]
    Think(String),

    #[error("Act error: {0}")]
    Act(String),

    #[error("Observe error: {0}")]
    Observe(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Summarize error: {0}")]
    Summarize(String),

    #[error("Tool error: {0}")]
    Tool(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_modes() {
        let full = PipelineMode::Full;
        assert_eq!(full.gates().len(), 8);

        let minimal = PipelineMode::Minimal;
        assert_eq!(minimal.gates().len(), 3);

        let none = PipelineMode::None;
        assert!(none.gates().is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_run() {
        let pipeline = AgentPipeline::new(PipelineMode::Minimal);

        let context = PipelineContext {
            user_id: "test_user".to_string(),
            session_id: "test_session".to_string(),
            mode: PipelineMode::Minimal,
            brain_files: vec![BrainFile {
                name: "SOUL.md".to_string(),
                content: "You are a helpful assistant.".to_string(),
            }],
            conversation_history: vec![],
            tools: vec![],
        };

        let state = PipelineState::new(context);
        let result = pipeline.run(state).await;

        assert!(result.is_ok());
        let final_state = result.unwrap();
        assert_eq!(final_state.current_stage, PipelineStage::Act);
    }
}
