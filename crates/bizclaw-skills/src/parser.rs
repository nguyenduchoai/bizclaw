//! SKILL.md parser — extracts metadata and content from skill files.

use serde::{Deserialize, Serialize};

/// Skill metadata from YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Unique skill name (lowercase, hyphenated).
    pub name: String,
    /// Human-readable display name.
    #[serde(default)]
    pub display_name: String,
    /// Short description.
    pub description: String,
    /// Version string (semver).
    #[serde(default = "default_version")]
    pub version: String,
    /// Author name.
    #[serde(default)]
    pub author: String,
    /// Categorization tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Category (e.g., "coding", "writing", "devops").
    #[serde(default)]
    pub category: String,
    /// Required tools for this skill.
    #[serde(default)]
    pub requires_tools: Vec<String>,
    /// Compatible providers.
    #[serde(default)]
    pub compatible_providers: Vec<String>,
    /// Icon emoji.
    #[serde(default = "default_icon")]
    pub icon: String,
}

fn default_version() -> String {
    "1.0.0".into()
}

fn default_icon() -> String {
    "📦".into()
}

/// A parsed skill with metadata and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Parsed metadata from frontmatter.
    pub metadata: SkillMetadata,
    /// The full markdown content (without frontmatter).
    pub content: String,
    /// Source path (if loaded from file).
    pub source_path: Option<String>,
    /// Download count (from marketplace).
    pub downloads: u64,
    /// Whether this skill is installed locally.
    pub installed: bool,
}

impl SkillManifest {
    /// Parse a SKILL.md file content into metadata + body.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let (metadata, content) = Self::split_frontmatter(raw)?;
        Ok(Self {
            metadata,
            content,
            source_path: None,
            downloads: 0,
            installed: false,
        })
    }

    /// Load from a file path.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("Read {}: {}", path.display(), e))?;
        let mut skill = Self::parse(&raw)?;
        skill.source_path = Some(path.to_string_lossy().to_string());
        skill.installed = true;
        Ok(skill)
    }

    /// Split YAML frontmatter from markdown body.
    fn split_frontmatter(raw: &str) -> Result<(SkillMetadata, String), String> {
        let trimmed = raw.trim();

        if !trimmed.starts_with("---") {
            return Err("SKILL.md must start with YAML frontmatter (---)".into());
        }

        let after_first = &trimmed[3..];
        let end_idx = after_first
            .find("---")
            .ok_or("Missing closing --- for frontmatter")?;

        let yaml_str = &after_first[..end_idx].trim();
        let body = after_first[end_idx + 3..].trim().to_string();

        // Parse YAML (we use serde_json via toml-like approach)
        // Simple YAML parser for frontmatter
        let metadata = Self::parse_yaml_frontmatter(yaml_str)?;

        Ok((metadata, body))
    }

    /// Simple YAML frontmatter parser.
    fn parse_yaml_frontmatter(yaml: &str) -> Result<SkillMetadata, String> {
        let mut name = String::new();
        let mut display_name = String::new();
        let mut description = String::new();
        let mut version = default_version();
        let mut author = String::new();
        let mut tags = Vec::new();
        let mut category = String::new();
        let mut requires_tools = Vec::new();
        let mut compatible_providers = Vec::new();
        let mut icon = default_icon();

        let mut current_list: Option<&str> = None;

        for line in yaml.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // List item
            if trimmed.starts_with("- ") {
                let value = trimmed[2..].trim().trim_matches('"').trim_matches('\'');
                match current_list {
                    Some("tags") => tags.push(value.to_string()),
                    Some("requires_tools") => requires_tools.push(value.to_string()),
                    Some("compatible_providers") => compatible_providers.push(value.to_string()),
                    _ => {}
                }
                continue;
            }

            current_list = None;

            if let Some((key, val)) = trimmed.split_once(':') {
                let key = key.trim();
                let val = val.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "name" => name = val.to_string(),
                    "display_name" => display_name = val.to_string(),
                    "description" => description = val.to_string(),
                    "version" => version = val.to_string(),
                    "author" => author = val.to_string(),
                    "category" => category = val.to_string(),
                    "icon" => icon = val.to_string(),
                    "tags" => {
                        if val.is_empty() {
                            current_list = Some("tags");
                        } else {
                            // Inline: tags: [a, b, c]
                            let inner = val.trim_matches(|c| c == '[' || c == ']');
                            tags = inner
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    }
                    "requires_tools" => {
                        if val.is_empty() {
                            current_list = Some("requires_tools");
                        }
                    }
                    "compatible_providers" => {
                        if val.is_empty() {
                            current_list = Some("compatible_providers");
                        }
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return Err("SKILL.md frontmatter must have a 'name' field".into());
        }
        if description.is_empty() {
            return Err("SKILL.md frontmatter must have a 'description' field".into());
        }
        if display_name.is_empty() {
            display_name = name.replace('-', " ");
            // Capitalize first letter of each word
            display_name = display_name
                .split_whitespace()
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + c.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
        }

        Ok(SkillMetadata {
            name,
            display_name,
            description,
            version,
            author,
            tags,
            category,
            requires_tools,
            compatible_providers,
            icon,
        })
    }

    /// Get estimated context size in tokens (rough: 1 token ≈ 4 chars).
    pub fn estimated_tokens(&self) -> usize {
        self.content.len() / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_md() {
        let raw = r#"---
name: rust-expert
display_name: Rust Expert
description: Deep expertise in Rust programming
version: "1.2.0"
author: BizClaw Team
category: coding
icon: 🦀
tags:
  - rust
  - programming
  - systems
requires_tools:
  - shell
  - file
  - edit_file
---

# Rust Expert Skill

You are an expert Rust programmer with deep knowledge of:
- Ownership and borrowing
- Async/await patterns
- Error handling with Result/Option
- Trait-based design
"#;

        let skill = SkillManifest::parse(raw).unwrap();
        assert_eq!(skill.metadata.name, "rust-expert");
        assert_eq!(skill.metadata.display_name, "Rust Expert");
        assert_eq!(skill.metadata.version, "1.2.0");
        assert_eq!(skill.metadata.category, "coding");
        assert_eq!(skill.metadata.icon, "🦀");
        assert_eq!(skill.metadata.tags.len(), 3);
        assert!(skill.metadata.tags.contains(&"rust".to_string()));
        assert_eq!(skill.metadata.requires_tools.len(), 3);
        assert!(skill.content.contains("Rust Expert Skill"));
        assert!(skill.estimated_tokens() > 0);
    }

    #[test]
    fn test_parse_minimal_skill() {
        let raw = r#"---
name: basic-skill
description: A basic skill
---

Some content here.
"#;
        let skill = SkillManifest::parse(raw).unwrap();
        assert_eq!(skill.metadata.name, "basic-skill");
        assert_eq!(skill.metadata.display_name, "Basic Skill");
        assert_eq!(skill.metadata.version, "1.0.0");
    }

    #[test]
    fn test_parse_missing_name() {
        let raw = r#"---
description: No name skill
---
Content
"#;
        assert!(SkillManifest::parse(raw).is_err());
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let raw = "Just plain content";
        assert!(SkillManifest::parse(raw).is_err());
    }

    #[test]
    fn test_inline_tags() {
        let raw = r#"---
name: test-skill
description: Test
tags: [web, api, rest]
---
Content
"#;
        let skill = SkillManifest::parse(raw).unwrap();
        assert_eq!(skill.metadata.tags, vec!["web", "api", "rest"]);
    }
}
