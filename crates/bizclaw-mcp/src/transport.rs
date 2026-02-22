//! Stdio transport for MCP — spawns a child process and communicates via JSON-RPC.

use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::types::{JsonRpcRequest, JsonRpcResponse};

/// Stdio transport — manages a child process for JSON-RPC communication.
pub struct StdioTransport {
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
}

impl StdioTransport {
    /// Spawn a new MCP server process.
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self, String> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())  // Suppress stderr to avoid polluting our output
            .kill_on_drop(true);

        let mut child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn MCP server '{}': {}", command, e))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| "Failed to take stdin".to_string())?;
        let stdout = child.stdout.take()
            .ok_or_else(|| "Failed to take stdout".to_string())?;

        Ok(Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
        })
    }

    /// Send a JSON-RPC request and read the response.
    pub async fn request(&mut self, req: &JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        // Serialize request + newline
        let mut json = serde_json::to_string(req)
            .map_err(|e| format!("Serialize error: {e}"))?;
        json.push('\n');

        // Write to stdin
        self.stdin.write_all(json.as_bytes()).await
            .map_err(|e| format!("Write error: {e}"))?;
        self.stdin.flush().await
            .map_err(|e| format!("Flush error: {e}"))?;

        // Read response line from stdout (with timeout)
        let mut line = String::new();
        let read_result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.reader.read_line(&mut line),
        ).await;

        match read_result {
            Ok(Ok(0)) => Err("MCP server closed stdout (EOF)".into()),
            Ok(Ok(_)) => {
                serde_json::from_str::<JsonRpcResponse>(&line)
                    .map_err(|e| format!("Parse response error: {e} — raw: {}", line.trim()))
            }
            Ok(Err(e)) => Err(format!("Read error: {e}")),
            Err(_) => Err("MCP server response timeout (30s)".into()),
        }
    }

    /// Check if the child process is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the child process.
    pub async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // kill_on_drop handles cleanup
    }
}
