//! Glob tool — find files matching patterns

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use std::path::Path;

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self { Self }
}

impl Default for GlobTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str { "glob" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "glob".into(),
            description: "Find files matching a glob pattern. Returns matching file paths with sizes. Example: **/*.rs, src/**/*.toml".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match (e.g., **/*.rs, src/**/*.toml, *.md)"
                    },
                    "directory": {
                        "type": "string",
                        "description": "Base directory to search from (default: current dir)"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 50)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let pattern = args["pattern"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'pattern'".into()))?;
        let directory = args["directory"].as_str().unwrap_or(".");
        let max_results = args["max_results"].as_u64().unwrap_or(50) as usize;

        // Build full pattern
        let full_pattern = if pattern.starts_with('/') || pattern.starts_with('.') {
            pattern.to_string()
        } else {
            format!("{}/{}", directory.trim_end_matches('/'), pattern)
        };

        // Use walkdir to find files matching the pattern
        let base = Path::new(directory);
        let mut results = Vec::new();

        // Walk directory tree
        let walker = walkdir_sync(base, 20);
        let pattern_re = glob_to_regex(pattern);

        for entry in walker {
            if results.len() >= max_results {
                break;
            }

            let path = &entry;
            let relative = path.strip_prefix(base)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            if pattern_re.is_match(&relative) || pattern_re.is_match(&path.to_string_lossy()) {
                let meta = std::fs::metadata(path).ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let kind = if path.is_dir() { "dir" } else { "file" };
                results.push(format!("{kind}\t{size}\t{relative}"));
            }
        }

        let output = if results.is_empty() {
            format!("No files matching pattern '{}' in {}", pattern, full_pattern)
        } else {
            format!("Found {} file(s) matching '{}':\n{}", results.len(), pattern, results.join("\n"))
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}

/// Simple recursive directory walker (sync, for tool use).
fn walkdir_sync(base: &Path, max_depth: usize) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    walk_recursive(base, 0, max_depth, &mut result);
    result
}

fn walk_recursive(dir: &Path, depth: usize, max_depth: usize, result: &mut Vec<std::path::PathBuf>) {
    if depth > max_depth { return; }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // Skip hidden directories except .bizclaw
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') && name != ".bizclaw" {
                continue;
            }
            // Skip common large/noisy directories
            if matches!(name, "node_modules" | "target" | ".git" | "__pycache__" | "dist" | "build") {
                continue;
            }
        }
        result.push(path.clone());
        if path.is_dir() {
            walk_recursive(&path, depth + 1, max_depth, result);
        }
    }
}

/// Convert a glob pattern to a regex pattern.
fn glob_to_regex(pattern: &str) -> regex::Regex {
    let mut regex_str = String::from("(?i)");
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    if chars.peek() == Some(&'/') {
                        chars.next(); // consume /
                        regex_str.push_str("(.*/)?");
                    } else {
                        regex_str.push_str(".*");
                    }
                } else {
                    regex_str.push_str("[^/]*");
                }
            }
            '?' => regex_str.push('.'),
            '.' => regex_str.push_str("\\."),
            '/' => regex_str.push('/'),
            _ => regex_str.push(c),
        }
    }

    regex::Regex::new(&format!("^{regex_str}$")).unwrap_or_else(|_| regex::Regex::new(".*").unwrap())
}
