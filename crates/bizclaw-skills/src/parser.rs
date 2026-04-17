//! SKILL.md parser — extracts metadata and content from skill files.

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// Skill metadata from YAML frontmatter.
/// Compatible with both BizClaw and OpenClaw/ClawHub SKILL.md format.
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
    /// Business category (e.g., "sales", "marketing", "support").
    #[serde(default)]
    pub business_category: String,
    /// Business roles that would use this skill.
    #[serde(default)]
    pub business_roles: Vec<String>,
    /// Industry tags (e.g., "retail", "ecommerce", "finance").
    #[serde(default)]
    pub industry: Vec<String>,
    /// Pain points this skill addresses.
    #[serde(default)]
    pub pain_points: Vec<String>,
    /// Required tools for this skill (BizClaw native).
    #[serde(default)]
    pub requires_tools: Vec<String>,
    /// Compatible providers.
    #[serde(default)]
    pub compatible_providers: Vec<String>,
    /// Icon emoji.
    #[serde(default = "default_icon")]
    pub icon: String,

    // ── OpenClaw/ClawHub-compatible fields ──
    /// Required environment variables (from metadata.openclaw.requires.env).
    #[serde(default)]
    pub requires_env: Vec<String>,
    /// Required CLI binaries (from metadata.openclaw.requires.bins).
    #[serde(default)]
    pub requires_bins: Vec<String>,
    /// Primary env var credential (from metadata.openclaw.primaryEnv).
    #[serde(default)]
    pub primary_env: String,
    /// Homepage URL (from metadata.openclaw.homepage).
    #[serde(default)]
    pub homepage: String,
    /// OS restrictions (from metadata.openclaw.os), e.g. ["macos", "linux"].
    #[serde(default)]
    pub os: Vec<String>,
    /// Source registry: "clawhub", "bizclaw", or "local".
    #[serde(default)]
    pub source: String,
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
        let raw =
            std::fs::read_to_string(path).map_err(|e| format!("Read {}: {}", path.display(), e))?;
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

    fn parse_yaml_frontmatter(yaml: &str) -> Result<SkillMetadata, String> {
        let value: Value = serde_yaml::from_str(yaml)
            .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?;

        let map = match &value {
            Value::Mapping(m) => m,
            _ => return Err("YAML frontmatter must be a mapping".into()),
        };

        fn get_str(map: &serde_yaml::Mapping, key: &str) -> String {
            map.get(&Value::String(key.to_string()))
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    if key == "version" {
                        "1.0.0".to_string()
                    } else {
                        String::new()
                    }
                })
        }

        fn get_vec_str(map: &serde_yaml::Mapping, key: &str) -> Vec<String> {
            map.get(&Value::String(key.to_string()))
                .and_then(|v| match v {
                    Value::Sequence(seq) => Some(
                        seq.iter()
                            .filter_map(|item| match item {
                                Value::String(s) => Some(s.clone()),
                                Value::Number(n) => Some(n.to_string()),
                                Value::Bool(b) => Some(b.to_string()),
                                _ => None,
                            })
                            .collect(),
                    ),
                    Value::String(s) => Some(
                        s.trim_matches(|c| c == '[' || c == ']')
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                    ),
                    _ => None,
                })
                .unwrap_or_default()
        }

        let name = get_str(map, "name");
        if name.is_empty() {
            return Err("SKILL.md frontmatter must have a 'name' field".into());
        }

        let description = get_str(map, "description");
        if description.is_empty() {
            return Err("SKILL.md frontmatter must have a 'description' field".into());
        }

        let mut display_name = get_str(map, "display_name");
        if display_name.is_empty() {
            display_name = name.replace('-', " ");
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
            version: get_str(map, "version"),
            author: get_str(map, "author"),
            tags: get_vec_str(map, "tags"),
            category: get_str(map, "category"),
            business_category: get_str(map, "business_category"),
            business_roles: get_vec_str(map, "business_roles"),
            industry: get_vec_str(map, "industry"),
            pain_points: get_vec_str(map, "pain_points"),
            requires_tools: get_vec_str(map, "requires_tools"),
            compatible_providers: get_vec_str(map, "compatible_providers"),
            icon: get_str(map, "icon"),
            requires_env: get_vec_str(map, "requires_env"),
            requires_bins: get_vec_str(map, "requires_bins"),
            primary_env: get_str(map, "primary_env"),
            homepage: get_str(map, "homepage"),
            os: get_vec_str(map, "os"),
            source: "local".to_string(),
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
