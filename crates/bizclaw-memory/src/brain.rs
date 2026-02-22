//! 3-Tier Memory system (3-tier brain architecture).
//!
//! ## 3 Tiers:
//! 1. **Brain MEMORY.md** — User-curated durable memory, loaded every turn (never touched by auto-compaction)
//! 2. **Daily Logs** — Auto-compaction summaries saved to `memory/YYYY-MM-DD.md`
//! 3. **FTS5 Search** — Keyword search across all stored conversations (hybrid search)
//!
//! ## Brain Workspace Files:
//! ```text
//! ~/.bizclaw/
//! ├── SOUL.md          # Personality, tone, behavioral rules
//! ├── IDENTITY.md      # Agent name, style, workspace path
//! ├── USER.md          # Who the human is
//! ├── MEMORY.md        # Long-term curated context (never auto-compacted)
//! ├── TOOLS.md         # Environment-specific notes
//! └── memory/          # Daily auto-compaction logs
//!     └── YYYY-MM-DD.md
//! ```

use bizclaw_core::error::Result;
use std::path::{Path, PathBuf};

/// Brain workspace — reads MD files to assemble dynamic system prompt.
pub struct BrainWorkspace {
    base_dir: PathBuf,
}

/// Brain file types that make up the dynamic system prompt.
const BRAIN_FILES: &[(&str, &str)] = &[
    ("SOUL.md", "PERSONALITY & RULES"),
    ("IDENTITY.md", "IDENTITY"),
    ("USER.md", "USER CONTEXT"),
    ("MEMORY.md", "LONG-TERM MEMORY"),
    ("TOOLS.md", "ENVIRONMENT NOTES"),
];

impl BrainWorkspace {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create workspace with default BizClaw home dir.
    pub fn default() -> Self {
        Self::new(bizclaw_core::config::BizClawConfig::home_dir())
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Assemble full brain context from workspace MD files.
    /// Files are re-read every turn (edit between messages = immediate effect).
    pub fn assemble_brain(&self) -> String {
        let mut brain = String::new();

        for (filename, section_name) in BRAIN_FILES {
            let path = self.base_dir.join(filename);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    brain.push_str(&format!("\n[{section_name}]\n{trimmed}\n[END {section_name}]\n"));
                }
            }
        }

        brain
    }

    /// Check which brain files exist.
    pub fn status(&self) -> Vec<(String, bool, u64)> {
        BRAIN_FILES.iter().map(|(filename, _)| {
            let path = self.base_dir.join(filename);
            let exists = path.exists();
            let size = if exists {
                std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };
            (filename.to_string(), exists, size)
        }).collect()
    }

    /// Initialize brain workspace with default files.
    pub fn initialize(&self) -> Result<()> {
        std::fs::create_dir_all(&self.base_dir)
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Create brain dir: {e}")))?;

        let defaults = [
            ("SOUL.md", "# Soul\nI am BizClaw, an AI assistant for business operations.\nI am helpful, precise, and action-oriented.\nI prefer to show code and results rather than lengthy explanations.\n"),
            ("IDENTITY.md", "# Identity\n- Name: BizClaw Agent\n- Role: AI Business Assistant\n- Workspace: ~/.bizclaw\n"),
            ("USER.md", "# User\n(Add information about yourself here — BizClaw reads this every turn)\n"),
            ("MEMORY.md", "# Long-Term Memory\n(Add important facts, preferences, and context here — this file is never touched by auto-compaction)\n"),
            ("TOOLS.md", "# Environment Notes\n(Add SSH hosts, API accounts, dev setup notes here)\n"),
        ];

        for (filename, content) in defaults {
            let path = self.base_dir.join(filename);
            if !path.exists() {
                std::fs::write(&path, content)
                    .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Write {filename}: {e}")))?;
            }
        }

        // Create memory directory for daily logs
        let memory_dir = self.base_dir.join("memory");
        std::fs::create_dir_all(&memory_dir)
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Create memory dir: {e}")))?;

        Ok(())
    }
}

/// Daily memory log manager — saves auto-compaction summaries.
pub struct DailyLogManager {
    memory_dir: PathBuf,
}

impl DailyLogManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let memory_dir = base_dir.join("memory");
        Self { memory_dir }
    }

    pub fn default() -> Self {
        Self::new(bizclaw_core::config::BizClawConfig::home_dir())
    }

    /// Save a compaction summary to today's daily log.
    /// Multiple compactions stack in the same file.
    pub fn save_compaction(&self, summary: &str) -> Result<()> {
        std::fs::create_dir_all(&self.memory_dir)
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Create memory dir: {e}")))?;

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let file_path = self.memory_dir.join(format!("{today}.md"));

        let timestamp = chrono::Utc::now().format("%H:%M:%S UTC").to_string();
        let entry = format!(
            "\n---\n## Compaction at {timestamp}\n\n{summary}\n",
        );

        // Append to existing file or create new
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Open log: {e}")))?;

        // If new file, add header
        if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            writeln!(file, "# Memory Log — {today}\n")
                .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Write header: {e}")))?;
        }

        write!(file, "{entry}")
            .map_err(|e| bizclaw_core::error::BizClawError::Memory(format!("Write entry: {e}")))?;

        tracing::info!("📝 Compaction summary saved to memory/{today}.md");
        Ok(())
    }

    /// List all daily log files.
    pub fn list_logs(&self) -> Vec<(String, u64)> {
        let mut logs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.memory_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".md") {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    logs.push((name, size));
                }
            }
        }
        logs.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
        logs
    }

    /// Read a specific daily log.
    pub fn read_log(&self, date: &str) -> Option<String> {
        let file_name = if date.ends_with(".md") {
            date.to_string()
        } else {
            format!("{date}.md")
        };
        let path = self.memory_dir.join(file_name);
        std::fs::read_to_string(path).ok()
    }

    /// Index all daily logs into the FTS5 memory database.
    /// Called on startup to ensure new logs are searchable.
    pub async fn index_into_memory(
        &self,
        memory: &dyn bizclaw_core::traits::memory::MemoryBackend,
    ) -> Result<()> {
        let logs = self.list_logs();
        let mut indexed = 0;

        for (filename, _size) in &logs {
            let path = self.memory_dir.join(filename);
            if let Ok(content) = std::fs::read_to_string(&path) {
                let id = format!("daily_log_{}", filename.trim_end_matches(".md"));

                // Check if already indexed
                if let Ok(Some(_)) = memory.get(&id).await {
                    continue; // Already indexed
                }

                let entry = bizclaw_core::traits::memory::MemoryEntry {
                    id,
                    content,
                    metadata: serde_json::json!({"type": "daily_log", "date": filename}),
                    embedding: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                if let Err(e) = memory.save(entry).await {
                    tracing::warn!("Failed to index daily log {}: {}", filename, e);
                } else {
                    indexed += 1;
                }
            }
        }

        if indexed > 0 {
            tracing::info!("📚 Indexed {} daily log(s) into memory", indexed);
        }
        Ok(())
    }

    /// Clean old logs (keep last N days).
    pub fn cleanup(&self, keep_days: usize) -> usize {
        let logs = self.list_logs();
        let mut removed = 0;
        for (i, (filename, _)) in logs.iter().enumerate() {
            if i >= keep_days {
                let path = self.memory_dir.join(filename);
                if std::fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
        if removed > 0 {
            tracing::info!("🧹 Cleaned {} old daily log(s)", removed);
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_brain_workspace_initialize() {
        let tmp = TempDir::new().unwrap();
        let ws = BrainWorkspace::new(tmp.path().to_path_buf());
        ws.initialize().unwrap();

        let status = ws.status();
        assert!(status.iter().all(|(_, exists, _)| *exists));
    }

    #[test]
    fn test_brain_workspace_assemble() {
        let tmp = TempDir::new().unwrap();
        let ws = BrainWorkspace::new(tmp.path().to_path_buf());
        ws.initialize().unwrap();

        let brain = ws.assemble_brain();
        assert!(brain.contains("[PERSONALITY & RULES]"));
        assert!(brain.contains("[IDENTITY]"));
        assert!(brain.contains("BizClaw"));
    }

    #[test]
    fn test_daily_log_manager() {
        let tmp = TempDir::new().unwrap();
        let mgr = DailyLogManager::new(tmp.path().to_path_buf());

        mgr.save_compaction("Test summary 1").unwrap();
        mgr.save_compaction("Test summary 2").unwrap();

        let logs = mgr.list_logs();
        assert_eq!(logs.len(), 1); // Same day = same file

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = mgr.read_log(&today).unwrap();
        assert!(content.contains("Test summary 1"));
        assert!(content.contains("Test summary 2"));
    }
}
