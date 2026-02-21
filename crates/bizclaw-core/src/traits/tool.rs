//! Tool trait — swappable tool execution.

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{ToolDefinition, ToolResult};

/// Tool trait — every executable tool implements this.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name.
    fn name(&self) -> &str;

    /// Tool definition for LLM function calling.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with given arguments.
    async fn execute(&self, arguments: &str) -> Result<ToolResult>;
}
