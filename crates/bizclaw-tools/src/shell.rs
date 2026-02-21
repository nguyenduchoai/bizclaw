//! Shell command execution tool.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

pub struct ShellTool;

impl ShellTool {
    pub fn new() -> Self { Self }
}

impl Default for ShellTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str { "shell" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell".into(),
            description: "Execute a shell command and return stdout/stderr.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "Working directory (optional)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let command = args["command"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'command'".into()))?;

        let workdir = args["workdir"].as_str();

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = if output.status.success() {
            stdout
        } else {
            format!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}\nExit code: {}", output.status.code().unwrap_or(-1))
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output: result,
            success: output.status.success(),
        })
    }
}
