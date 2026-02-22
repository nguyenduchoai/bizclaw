//! Grep tool — search file contents with regex

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use std::path::Path;

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self { Self }
}

impl Default for GrepTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep".into(),
            description: "Search file contents with regex or literal text. Returns matching lines with file paths and line numbers.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (regex or literal text)"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search in (default: current dir)"
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "Case-insensitive search (default: true)"
                    },
                    "include": {
                        "type": "string",
                        "description": "File extension filter (e.g., 'rs', 'py', 'js')"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum matches to return (default: 30)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(e.to_string()))?;

        let pattern_str = args["pattern"].as_str()
            .ok_or_else(|| bizclaw_core::error::BizClawError::Tool("Missing 'pattern'".into()))?;
        let path = args["path"].as_str().unwrap_or(".");
        let case_insensitive = args["case_insensitive"].as_bool().unwrap_or(true);
        let include = args["include"].as_str();
        let max_results = args["max_results"].as_u64().unwrap_or(30) as usize;

        // Build regex
        let pattern = if case_insensitive {
            format!("(?i){}", regex::escape(pattern_str))
        } else {
            regex::escape(pattern_str)
        };
        let re = regex::Regex::new(&pattern)
            .map_err(|e| bizclaw_core::error::BizClawError::Tool(format!("Invalid pattern: {e}")))?;

        let root = Path::new(path);
        let mut matches = Vec::new();

        if root.is_file() {
            search_file(root, &re, &mut matches, max_results);
        } else if root.is_dir() {
            search_dir(root, &re, include, &mut matches, max_results, 0, 10);
        } else {
            return Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("Path not found: {path}"),
                success: false,
            });
        }

        let output = if matches.is_empty() {
            format!("No matches for '{}' in {}", pattern_str, path)
        } else {
            let mut out = format!("Found {} match(es) for '{}':\n\n", matches.len(), pattern_str);
            for m in &matches {
                out.push_str(m);
                out.push('\n');
            }
            if matches.len() >= max_results {
                out.push_str(&format!("\n... (results capped at {max_results})"));
            }
            out
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}

fn search_file(path: &Path, re: &regex::Regex, matches: &mut Vec<String>, max: usize) {
    if matches.len() >= max { return; }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return, // skip binary/unreadable files
    };

    let path_str = path.to_string_lossy();

    for (line_num, line) in content.lines().enumerate() {
        if matches.len() >= max { break; }
        if re.is_match(line) {
            let display_line = if line.len() > 200 {
                format!("{}...", &line[..200])
            } else {
                line.to_string()
            };
            matches.push(format!("{}:{}: {}", path_str, line_num + 1, display_line));
        }
    }
}

fn search_dir(
    dir: &Path,
    re: &regex::Regex,
    include: Option<&str>,
    matches: &mut Vec<String>,
    max: usize,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth || matches.len() >= max { return; }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if matches.len() >= max { break; }

        let path = entry.path();

        // Skip hidden/noisy directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
            if matches!(name, "node_modules" | "target" | "__pycache__" | "dist" | "build" | ".git") {
                continue;
            }
        }

        if path.is_dir() {
            search_dir(&path, re, include, matches, max, depth + 1, max_depth);
        } else if path.is_file() {
            // Check extension filter
            if let Some(ext_filter) = include {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext != ext_filter {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            search_file(&path, re, matches, max);
        }
    }
}
