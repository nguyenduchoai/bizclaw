//! Persistent Bash Tool — stateful terminal execution matching Claude Code.
//!
//! Unlike `ShellTool` which spawns a fresh `sh -c` for every command,
//! this tool maintains a continuous `bash` session. Variables, aliases,
//! current directory (`cd`), and background processes persist across tool calls!

use async_trait::async_trait;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use uuid::Uuid;

use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

struct PersistentSession {
    #[allow(dead_code)] // Keep process alive
    child: Child,
    stdin: ChildStdin,
    stdout_reader: BufReader<ChildStdout>,
}

impl PersistentSession {
    fn spawn() -> std::io::Result<Self> {
        let mut child = Command::new("bash")
            // Disable history formatting, keep it clean
            .arg("--noprofile")
            .arg("--norc")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().expect("Failed to open child stdin");
        let stdout = child.stdout.take().expect("Failed to open child stdout");
        // Combine stderr into stdout for easier parsing, or redirect stderr in the bash command.
        // We will just handle it via `exec 2>&1` in init.

        let stdout_reader = BufReader::new(stdout);

        Ok(Self {
            child,
            stdin,
            stdout_reader,
        })
    }

    async fn execute(&mut self, command: &str, timeout_secs: u64) -> Result<String> {
        // Delimiter to know when the command finishes
        let delim = format!("BIZCLAW_EOF_{}", Uuid::new_v4().simple());

        // Ensure stderr is combined into stdout for this session
        // Then run the command, capture exit code, print exit code and delimiter
        let script = format!(
            "exec 2>&1\n{}\nBZC_CMD_EXIT=$?\necho \"\n{} $BZC_CMD_EXIT\"\n",
            command, delim
        );

        // Write the script to bash stdin
        if let Err(e) = self.stdin.write_all(script.as_bytes()).await {
            return Ok(format!("Failed to write to bash stdin: {}", e));
        }
        if let Err(e) = self.stdin.flush().await {
            return Ok(format!("Failed to flush bash stdin: {}", e));
        }

        let mut output = String::new();
        let mut exit_code = 0;
        let mut lines_read = 0;

        // Read stdout line by line until delimiter is found, with a timeout
        let read_future = async {
            let mut line = String::new();
            loop {
                line.clear();
                match self.stdout_reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let trimmed = line.trim_end();
                        if trimmed.starts_with(&delim) {
                            // Extract exit code
                            if let Some(code_str) = trimmed.split_whitespace().nth(1) {
                                exit_code = code_str.parse().unwrap_or(0);
                            }
                            break;
                        }
                        output.push_str(&line);
                        lines_read += 1;
                        if lines_read > 50000 {
                            output.push_str("\n...[truncated output too large]");
                            break;
                        }
                    }
                    Err(e) => {
                        output.push_str(&format!("\n[Error reading stdout: {}]", e));
                        break;
                    }
                }
            }
        };

        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), read_future).await
        {
            Ok(_) => {
                let trimmed_out = output.trim().to_string();
                if exit_code == 0 {
                    Ok(trimmed_out)
                } else {
                    Ok(format!("(Exit code: {})\n{}", exit_code, trimmed_out))
                }
            }
            Err(_) => {
                // Timeout! Send Ctrl+C via SIGINT (we cannot easily send SIGINT to child in rust cross-platform,
                // but we can just return the partial output).
                Ok(format!(
                    "{}\n\n[Command timed out after {}s. It may be running in the background.]",
                    output.trim(),
                    timeout_secs
                ))
            }
        }
    }
}

pub struct PersistentBashTool {
    session: Arc<Mutex<PersistentSession>>,
}

impl PersistentBashTool {
    pub fn new() -> Self {
        let session = PersistentSession::spawn().expect("Failed to spawn persistent bash");
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }
}

impl Default for PersistentBashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PersistentBashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bash".into(),
            description: "Run commands in a persistent Bash session. Variables, aliases, and directory changes (`cd`) persist across calls. Supports background processes. Ideal for complex coding tasks where state matters.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 120)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let command = args["command"].as_str().unwrap_or("");
        if command.trim().is_empty() {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: "Empty command".into(),
                success: false,
            });
        }

        let timeout = args["timeout_secs"].as_u64().unwrap_or(120);

        tracing::info!(
            "💻 Bash: executing `{}`",
            bizclaw_core::safe_truncate(command, 80)
        );

        let mut session = self.session.lock().await;
        let output = session.execute(command, timeout).await?;

        // Simple heuristic for success (if it doesn't say "Exit code")
        let success = !output.contains("(Exit code: ");

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success,
        })
    }
}
