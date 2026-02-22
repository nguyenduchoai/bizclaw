//! Execute Code tool — run code in various languages
//!
//! Supports: Python, JavaScript/Node, Ruby, Go, Rust, C, PHP, Bash

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

pub struct ExecuteCodeTool;

impl ExecuteCodeTool {
    pub fn new() -> Self { Self }
}

impl Default for ExecuteCodeTool {
    fn default() -> Self { Self::new() }
}

struct LangConfig {
    command: &'static str,
    args: Vec<String>,
    extension: &'static str,
    needs_compile: bool,
}

fn get_lang_config(language: &str) -> Option<LangConfig> {
    match language.to_lowercase().as_str() {
        "python" | "py" | "python3" => Some(LangConfig {
            command: "python3",
            args: vec![],
            extension: "py",
            needs_compile: false,
        }),
        "javascript" | "js" | "node" => Some(LangConfig {
            command: "node",
            args: vec![],
            extension: "js",
            needs_compile: false,
        }),
        "ruby" | "rb" => Some(LangConfig {
            command: "ruby",
            args: vec![],
            extension: "rb",
            needs_compile: false,
        }),
        "bash" | "sh" | "shell" => Some(LangConfig {
            command: "bash",
            args: vec![],
            extension: "sh",
            needs_compile: false,
        }),
        "php" => Some(LangConfig {
            command: "php",
            args: vec![],
            extension: "php",
            needs_compile: false,
        }),
        "go" | "golang" => Some(LangConfig {
            command: "go",
            args: vec!["run".to_string()],
            extension: "go",
            needs_compile: false,
        }),
        "rust" | "rs" => Some(LangConfig {
            command: "rustc",
            args: vec![],
            extension: "rs",
            needs_compile: true,
        }),
        "c" => Some(LangConfig {
            command: "gcc",
            args: vec!["-o".to_string()],
            extension: "c",
            needs_compile: true,
        }),
        "typescript" | "ts" => Some(LangConfig {
            command: "npx",
            args: vec!["tsx".to_string()],
            extension: "ts",
            needs_compile: false,
        }),
        _ => None,
    }
}

#[async_trait]
impl Tool for ExecuteCodeTool {
    fn name(&self) -> &str { "execute_code" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "execute_code".into(),
            description: "Execute code in various programming languages. Writes code to a temp file, runs it, and returns stdout/stderr. Supports: python, javascript, ruby, bash, php, go, rust, c, typescript.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["python", "javascript", "ruby", "bash", "php", "go", "rust", "c", "typescript"],
                        "description": "Programming language"
                    },
                    "code": {
                        "type": "string",
                        "description": "Source code to execute"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Execution timeout in seconds (default: 30, max: 120)"
                    }
                },
                "required": ["language", "code"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let language = args["language"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'language'".into()))?;
        let code = args["code"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'code'".into()))?;
        let timeout = args["timeout_secs"].as_u64().unwrap_or(30).min(120);

        let config = get_lang_config(language)
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool(
                format!("Unsupported language: {}. Supported: python, javascript, ruby, bash, php, go, rust, c, typescript", language)
            ))?;

        // Write code to temp file
        let temp_dir = std::env::temp_dir().join("bizclaw_exec");
        tokio::fs::create_dir_all(&temp_dir).await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Create temp dir: {e}")))?;

        let file_name = format!("exec_{}.{}", uuid::Uuid::new_v4().to_string()[..8].to_string(), config.extension);
        let file_path = temp_dir.join(&file_name);
        tokio::fs::write(&file_path, code).await
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Write temp file: {e}")))?;

        let start = std::time::Instant::now();

        let output = if config.needs_compile {
            // Compile then run
            let out_path = temp_dir.join(format!("exec_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()));

            let compile_output = if config.command == "rustc" {
                tokio::process::Command::new(config.command)
                    .arg(file_path.to_str().unwrap())
                    .arg("-o")
                    .arg(out_path.to_str().unwrap())
                    .output()
                    .await
            } else {
                // gcc
                tokio::process::Command::new(config.command)
                    .arg(file_path.to_str().unwrap())
                    .arg("-o")
                    .arg(out_path.to_str().unwrap())
                    .output()
                    .await
            };

            match compile_output {
                Ok(co) if !co.status.success() => {
                    let stderr = String::from_utf8_lossy(&co.stderr);
                    let _ = tokio::fs::remove_file(&file_path).await;
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Compilation failed:\n{}", stderr),
                        success: false,
                    });
                }
                Err(e) => {
                    let _ = tokio::fs::remove_file(&file_path).await;
                    return Ok(ToolResult {
                        tool_call_id: String::new(),
                        output: format!("Compiler not found ({}): {}", config.command, e),
                        success: false,
                    });
                }
                _ => {}
            }

            let run = tokio::time::timeout(
                std::time::Duration::from_secs(timeout),
                tokio::process::Command::new(out_path.to_str().unwrap())
                    .output()
            ).await;

            let _ = tokio::fs::remove_file(&file_path).await;
            let _ = tokio::fs::remove_file(&out_path).await;
            run
        } else {
            // Interpreted — just run
            let mut cmd_args: Vec<String> = config.args;
            cmd_args.push(file_path.to_str().unwrap().to_string());

            let run = tokio::time::timeout(
                std::time::Duration::from_secs(timeout),
                tokio::process::Command::new(config.command)
                    .args(&cmd_args)
                    .output()
            ).await;

            let _ = tokio::fs::remove_file(&file_path).await;
            run
        };

        let elapsed = start.elapsed();

        match output {
            Ok(Ok(o)) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);

                let mut result = format!("Language: {} | Exit: {} | Time: {:.1}s\n",
                    language, o.status.code().unwrap_or(-1), elapsed.as_secs_f64());

                if !stdout.is_empty() {
                    let stdout_display = if stdout.len() > 5000 {
                        format!("{}...\n[truncated, {} bytes total]", &stdout[..5000], stdout.len())
                    } else {
                        stdout.to_string()
                    };
                    result.push_str(&format!("\nSTDOUT:\n{}", stdout_display));
                }
                if !stderr.is_empty() {
                    let stderr_display = if stderr.len() > 2000 {
                        format!("{}...\n[truncated]", &stderr[..2000])
                    } else {
                        stderr.to_string()
                    };
                    result.push_str(&format!("\nSTDERR:\n{}", stderr_display));
                }
                if stdout.is_empty() && stderr.is_empty() {
                    result.push_str("\n(no output)");
                }

                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: result,
                    success: o.status.success(),
                })
            }
            Ok(Err(e)) => {
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("Execution failed — '{}' not found or not executable: {}", config.command, e),
                    success: false,
                })
            }
            Err(_) => {
                Ok(ToolResult {
                    tool_call_id: String::new(),
                    output: format!("⏰ Execution timed out after {}s", timeout),
                    success: false,
                })
            }
        }
    }
}
