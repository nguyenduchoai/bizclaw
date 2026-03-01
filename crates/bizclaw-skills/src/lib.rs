//! # BizClaw Skills
//!
//! Skills marketplace — SKILL.md parser, registry, discovery, and installation.
//!
//! ## Skill Format
//! A skill is a directory containing:
//! - `SKILL.md` — Markdown with YAML frontmatter (name, description, version, tags)
//! - Optional asset files (scripts, templates, data)
//!
//! ## Marketplace
//! Skills can be installed from:
//! - Built-in skills (bundled with BizClaw)
//! - Local directories
//! - Remote URL (BizClaw Hub)

pub mod parser;
pub mod registry;
pub mod marketplace;
pub mod builtin;

pub use parser::{SkillManifest, SkillMetadata};
pub use registry::SkillRegistry;
pub use marketplace::SkillMarketplace;
