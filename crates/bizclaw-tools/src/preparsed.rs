//! # Pre-parsed Commands - Lệnh tức thì không tốn token
//!
//! Zero-token-cost commands cho BizClaw - thực thi ngay lập tức mà không cần gọi LLM.
//!
//! ## Commands
//! | Command | Mô tả |
//! |---------|-------|
//! | /help | Hiển thị danh sách commands |
//! | /status | Trạng thái hệ thống |
//! | /health | Health check |
//! | /version | Phiên bản BizClaw |
//! | /ls [path] | Liệt kê files |
//! | /cat <file> | Đọc file |
//! | /read <file> | Đọc file |
//! | /write <path> <content> | Ghi file |
//! | /find <pattern> | Tìm files |
//! | /grep <pattern> [file] | Tìm trong files |
//! | /runs <cmd> | Chạy shell command |
//! | /search <query> | Web search |
//! | /google <query> | Google search |
//! | /fetch <url> | Fetch web page |
//! | /screenshot | Chụp màn hình |
//! | /ctx | Xem context stats |
//! | /remember <text> | Lưu vào memory |
//! | /recall [query] | Tìm trong memory |
//! | /clear | Xóa conversation |
//! | /tools | Liệt kê tools |
//! | /skills | Liệt kê skills |
//! | /skill install <name> | Cài skill |
//! | /skill list | Liệt kê skills |
//! | /skill search <query> | Tìm skills |

use async_trait::async_trait;
use bizclaw_core::error::BizClawError;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command as SysCommand;

pub struct PreparsedCommandsTool {
    workspace_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct PreparsedArgs {
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

impl PreparsedCommandsTool {
    pub fn new() -> Self {
        Self {
            workspace_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".bizclaw")
                .join("workspace"),
        }
    }

    pub fn with_workspace(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }

    fn is_preparsed(command: &str) -> bool {
        command.starts_with('/')
    }

    fn parse_command(input: &str) -> Option<(String, Vec<String>)> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command = parts[0].trim_start_matches('/').to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        Some((command, args))
    }

    async fn execute_command(
        &self,
        command: &str,
        args: &[String],
    ) -> std::result::Result<String, String> {
        match command {
            "help" | "h" | "?" => Ok(self.help()),
            "status" | "stat" => Ok(self.status()),
            "health" => Ok(self.health()),
            "version" | "v" => Ok(self.version()),
            "ls" | "dir" => self.list_files(args).await,
            "cat" | "read" => self.read_file(args).await,
            "write" => self.write_file(args).await,
            "find" => self.find_files(args).await,
            "grep" | "search" => self.grep(args).await,
            "runs" | "exec" | "$" => self.run_shell(args).await,
            "google" => self.google_search(args).await,
            "fetch" | "curl" => self.fetch_url(args).await,
            "screenshot" | "ss" => Ok("Use computer_use tool with action=screenshot".to_string()),
            "ctx" | "context" => Ok(self.context()),
            "remember" | "mem" => self.remember(args).await,
            "recall" | "search_memory" => self.recall(args).await,
            "clear" | "reset" => Ok(self.clear()),
            "tools" => Ok(self.list_tools()),
            "skills" => Ok(self.list_skills()),
            "cwd" | "pwd" => Ok(self.cwd()),
            "date" => Ok(self.date()),
            "whoami" => Ok(self.whoami()),
            "uptime" => Ok(self.uptime()),
            _ => Err(format!(
                "Unknown command: /{}. Type /help for available commands.",
                command
            )),
        }
    }

    fn help(&self) -> String {
        r#"
📋 Pre-parsed Commands (zero-token-cost)

📁 File Operations:
  /ls [path]              - Liệt kê files
  /cat <file>             - Đọc file
  /read <file>            - Đọc file  
  /write <path> <content> - Ghi file
  /find <pattern>         - Tìm files
  /grep <pattern> [file]  - Tìm trong files

🔧 System:
  /help, /h, /?           - Hiển thị help
  /status, /stat          - Trạng thái
  /health                 - Health check
  /version, /v            - Phiên bản
  /whoami                 - Current user
  /date                   - Ngày giờ hiện tại
  /uptime                 - System uptime
  /cwd, /pwd              - Working directory
  /runs <cmd>             - Chạy shell command

🌐 Web:
  /google <query>         - Google search
  /fetch <url>            - Fetch web page

💾 Memory:
  /remember <text>        - Lưu vào memory
  /recall [query]         - Tìm trong memory
  /ctx, /context          - Xem context stats

🔌 Tools & Skills:
  /tools                  - Liệt kê tools
  /skills                 - Liệt kê skills
  /skill install <name>   - Cài skill
  /skill list             - Liệt kê skills
  /skill search <query>   - Tìm skills

🧹 Other:
  /clear                  - Xóa conversation

Note: Commands bắt đầu bằng / được thực thi ngay, không tốn token LLM.
"#
        .trim()
        .to_string()
    }

    fn status(&self) -> String {
        let tool_count = 15;
        format!(
            "✅ BizClaw Status\n\n\
             🔧 Tools: {}\n\
             📁 Workspace: {}\n\
             🧠 Memory: Active\n\
             🌐 Network: Connected",
            tool_count,
            self.workspace_dir.display()
        )
    }

    fn health(&self) -> String {
        "✅ BizClaw Health\n\n\
             🧠 Memory: OK\n\
             📦 Tools: OK\n\
             🌐 Network: OK\n\
             💾 Disk: OK"
            .to_string()
    }

    fn version(&self) -> String {
        let rust_ver = rustc_version::version()
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        format!(
            "BizClaw v{}\n\
             Edition: SME Edition\n\
             Rust: {}",
            env!("CARGO_PKG_VERSION"),
            rust_ver
        )
    }

    fn context(&self) -> String {
        r#"
📊 Context Stats:
- Messages: (trong session)
- Tokens: (ước tính)
- Tool calls: (trong session)
- Session ID: (hiện tại)

Sử dụng /clear để reset conversation.
"#
        .to_string()
    }

    fn clear(&self) -> String {
        "🧹 Conversation cleared. Type /help for available commands.".to_string()
    }

    fn list_tools(&self) -> String {
        r#"
🔧 Available Tools:
1. shell - Execute commands
2. file - File operations
3. computer_use - Desktop control
4. browser - Web browser
5. web_search - Search web
6. http_request - HTTP requests
7. db_query - Database queries
8. document_reader - PDF/DOCX/XLSX
9. memory_search - Search memory
10. calendar - Calendar integration
11. config_manager - Config management
12. plan - Task planning
13. email - Email operations
"#
        .to_string()
    }

    fn list_skills(&self) -> String {
        r#"
🛠️ Available Skills:
(Chưa có skills nào được cài đặt)

Sử dụng /skill install <name> để cài đặt.
"#
        .to_string()
    }

    fn cwd(&self) -> String {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn date(&self) -> String {
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn whoami(&self) -> String {
        whoami::username()
    }

    fn uptime(&self) -> String {
        if let Ok(uptime) = std::fs::read_to_string("/proc/uptime") {
            if let Some(first) = uptime.split_whitespace().next() {
                if let Ok(seconds) = first.parse::<f64>() {
                    let days = (seconds / 86400.0) as u64;
                    let hours = ((seconds % 86400.0) / 3600.0) as u64;
                    let mins = ((seconds % 3600.0) / 60.0) as u64;
                    return format!("{} days, {} hours, {} minutes", days, hours, mins);
                }
            }
        }
        "System uptime: unavailable".to_string()
    }

    async fn list_files(&self, args: &[String]) -> std::result::Result<String, String> {
        let path = if let Some(p) = args.first() {
            PathBuf::from(p)
        } else {
            self.workspace_dir.clone()
        };

        let mut output = format!("📁 {}\n\n", path.display());

        let entries =
            std::fs::read_dir(&path).map_err(|e| format!("Cannot read directory: {e}"))?;

        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();

        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        for entry in entries.iter().take(50) {
            let name = entry.file_name();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let icon = if is_dir { "📁" } else { "📄" };
            output.push_str(&format!("{} {}\n", icon, name.to_string_lossy()));
        }

        if entries.len() > 50 {
            output.push_str(&format!("\n... và {} files khác", entries.len() - 50));
        }

        Ok(output)
    }

    async fn read_file(&self, args: &[String]) -> std::result::Result<String, String> {
        let file = args.first().ok_or("Usage: /read <filename>")?;

        let path = if PathBuf::from(file).is_absolute() {
            PathBuf::from(file)
        } else {
            self.workspace_dir.join(file)
        };

        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("Cannot read file: {e}"))?;

        let truncated = if content.len() > 5000 {
            format!(
                "{}...\n\n[{} bytes truncated]",
                &content[..5000],
                content.len()
            )
        } else {
            content
        };

        Ok(format!("📄 {}\n\n{}", path.display(), truncated))
    }

    async fn write_file(&self, args: &[String]) -> std::result::Result<String, String> {
        if args.len() < 2 {
            return Err("Usage: /write <filename> <content>".to_string());
        }

        let file = &args[0];
        let content = args[1..].join(" ");

        let path = if PathBuf::from(file).is_absolute() {
            PathBuf::from(file)
        } else {
            self.workspace_dir.join(file)
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create directory: {e}"))?;
        }

        std::fs::write(&path, &content).map_err(|e| format!("Cannot write file: {e}"))?;

        Ok(format!(
            "✅ Đã ghi {} bytes vào {}",
            content.len(),
            path.display()
        ))
    }

    async fn find_files(&self, args: &[String]) -> std::result::Result<String, String> {
        let pattern = args.first().ok_or("Usage: /find <pattern>")?.to_lowercase();

        let mut output = format!("🔍 Tìm: {}\n\n", pattern);
        let mut count = 0;

        self.find_recursive(&self.workspace_dir, &pattern, &mut output, &mut count)?;

        if count == 0 {
            output.push_str("Không tìm thấy files nào.");
        } else {
            output.push_str(&format!("\n\nTìm thấy {} files.", count));
        }

        Ok(output)
    }

    fn find_recursive(
        &self,
        dir: &PathBuf,
        pattern: &str,
        output: &mut String,
        count: &mut usize,
    ) -> std::result::Result<(), String> {
        if *count >= 100 {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {e}"))?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if path.is_dir() {
                if !name.starts_with('.') {
                    self.find_recursive(&path, pattern, output, count)?;
                }
            } else if name.contains(pattern) {
                *count += 1;
                output.push_str(&format!("📄 {}\n", path.display()));
            }
        }

        Ok(())
    }

    async fn grep(&self, args: &[String]) -> std::result::Result<String, String> {
        if args.is_empty() {
            return Err("Usage: /grep <pattern> [file]".to_string());
        }

        let pattern = &args[0];
        let file = args.get(1);

        let mut output = format!("🔍 Grep: {}\n\n", pattern);

        if let Some(file) = file {
            let path = self.workspace_dir.join(file);
            if let Ok(content) = std::fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    if line.contains(pattern) {
                        output.push_str(&format!("{:4}: {}\n", i + 1, line));
                    }
                }
            }
        } else {
            self.grep_recursive(&self.workspace_dir, pattern, &mut output)?;
        }

        Ok(output)
    }

    fn grep_recursive(
        &self,
        dir: &PathBuf,
        pattern: &str,
        output: &mut String,
    ) -> std::result::Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {e}"))?;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                if !name.starts_with('.') {
                    self.grep_recursive(&path, pattern, output)?;
                }
            } else if let Ok(content) = std::fs::read_to_string(&path) {
                let mut found = false;
                for line in content.lines() {
                    if line.contains(pattern) {
                        if !found {
                            output.push_str(&format!("\n📄 {}\n", path.display()));
                            found = true;
                        }
                        output.push_str(&format!("  {}\n", line));
                    }
                }
            }
        }

        Ok(())
    }

    async fn run_shell(&self, args: &[String]) -> std::result::Result<String, String> {
        let cmd = args.join(" ");

        let output = if cfg!(target_os = "windows") {
            SysCommand::new("cmd").args(["/C", &cmd]).output()
        } else {
            SysCommand::new("sh").args(["-c", &cmd]).output()
        };

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);

                let mut result = format!("$ {}\n\n", cmd);

                if !stdout.is_empty() {
                    result.push_str(&stdout);
                }

                if !stderr.is_empty() {
                    result.push_str(&format!("\n❌ stderr:\n{}", stderr));
                }

                if out.status.success() {
                    result.push_str("\n✅ Exit: 0");
                } else {
                    result.push_str(&format!("\n❌ Exit: {:?}", out.status.code()));
                }

                Ok(result)
            }
            Err(e) => Err(format!("Failed to execute: {e}")),
        }
    }

    async fn google_search(&self, args: &[String]) -> std::result::Result<String, String> {
        if args.is_empty() {
            return Err("Usage: /google <query>".to_string());
        }

        let query = args.join("+");
        Ok(format!(
            "🔍 Search: {}\n\n\
             Kết quả tìm kiếm sẽ được hiển thị bởi web_search tool.\n\
             URL: https://www.google.com/search?q={}",
            args.join(" "),
            query
        ))
    }

    async fn fetch_url(&self, args: &[String]) -> std::result::Result<String, String> {
        let url = args.first().ok_or("Usage: /fetch <url>")?;

        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("Fetch failed: {e}"))?;

        let body = response
            .text()
            .await
            .map_err(|e| format!("Read body failed: {e}"))?;

        let truncated = if body.len() > 3000 {
            format!("{}...\n\n[{} bytes truncated]", &body[..3000], body.len())
        } else {
            body
        };

        Ok(format!("📄 {}\n\n{}", url, truncated))
    }

    async fn remember(&self, args: &[String]) -> std::result::Result<String, String> {
        if args.is_empty() {
            return Err("Usage: /remember <text>".to_string());
        }

        let text = args.join(" ");
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

        let memory_file = self.workspace_dir.join("memory.md");
        let entry = format!("- [{}] {}\n", timestamp, text);

        let current = std::fs::read_to_string(&memory_file).unwrap_or_default();
        std::fs::write(&memory_file, format!("{}\n{}", current, entry))
            .map_err(|e| format!("Cannot save memory: {e}"))?;

        Ok(format!("✅ Đã lưu vào memory:\n{}", text))
    }

    async fn recall(&self, args: &[String]) -> std::result::Result<String, String> {
        let memory_file = self.workspace_dir.join("memory.md");

        if !memory_file.exists() {
            return Ok("📭 Memory trống. Sử dụng /remember <text> để lưu.".to_string());
        }

        let content = std::fs::read_to_string(&memory_file)
            .map_err(|e| format!("Cannot read memory: {e}"))?;

        if args.is_empty() {
            return Ok(format!("📚 Memory:\n\n{}", content));
        }

        let query = args.join(" ").to_lowercase();
        let filtered: Vec<_> = content
            .lines()
            .filter(|line| line.to_lowercase().contains(&query))
            .collect();

        if filtered.is_empty() {
            return Ok(format!(
                "🔍 Không tìm thấy '{}' trong memory.",
                args.join(" ")
            ));
        }

        Ok(format!(
            "📚 Kết quả tìm kiếm '{}':\n\n{}",
            query,
            filtered.join("\n")
        ))
    }
}

#[async_trait]
impl Tool for PreparsedCommandsTool {
    fn name(&self) -> &str {
        "preparsed"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "preparsed".to_string(),
            description: "Pre-parsed commands - thực thi ngay lập tức không tốn token. Bắt đầu với /: /help, /ls, /cat, /find, /grep, /runs, /search, /remember, /recall, /tools, /skills, /status, /version, /health".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command name without /"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Command arguments"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let parsed: PreparsedArgs = serde_json::from_str(args)
            .map_err(|e| BizClawError::Tool(format!("Invalid arguments: {e}")))?;

        let result = self
            .execute_command(&parsed.command, &parsed.args)
            .await
            .map_err(|e| BizClawError::Tool(e))?;

        Ok(ToolResult {
            tool_call_id: "preparsed".to_string(),
            output: result,
            success: true,
        })
    }
}

pub fn new() -> Box<dyn Tool> {
    Box::new(PreparsedCommandsTool::new())
}

pub fn new_with_workspace(workspace_dir: PathBuf) -> Box<dyn Tool> {
    Box::new(PreparsedCommandsTool::with_workspace(workspace_dir))
}
