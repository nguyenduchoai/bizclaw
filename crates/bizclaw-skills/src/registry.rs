//! Skill registry — manages installed skills with search and discovery.

use std::collections::HashMap;
use tracing::info;

use crate::parser::SkillManifest;

/// Registry of installed skills.
pub struct SkillRegistry {
    skills: HashMap<String, SkillManifest>,
}

impl SkillRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Create registry with built-in skills.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        for skill in crate::builtin::builtin_skills() {
            reg.install(skill);
        }
        reg
    }

    /// Install a skill.
    pub fn install(&mut self, skill: SkillManifest) {
        info!("📦 Installed skill: {} v{}", skill.metadata.name, skill.metadata.version);
        self.skills.insert(skill.metadata.name.clone(), skill);
    }

    /// Uninstall a skill.
    pub fn uninstall(&mut self, name: &str) -> bool {
        self.skills.remove(name).is_some()
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<&SkillManifest> {
        self.skills.get(name)
    }

    /// Get skill content for context injection.
    pub fn get_content(&self, name: &str) -> Option<&str> {
        self.skills.get(name).map(|s| s.content.as_str())
    }

    /// List all installed skills.
    pub fn list(&self) -> Vec<&SkillManifest> {
        self.skills.values().collect()
    }

    /// Count installed skills.
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Search skills by keyword (matches name, description, tags).
    pub fn search(&self, query: &str) -> Vec<&SkillManifest> {
        let q = query.to_lowercase();
        self.skills
            .values()
            .filter(|s| {
                s.metadata.name.to_lowercase().contains(&q)
                    || s.metadata.description.to_lowercase().contains(&q)
                    || s.metadata.display_name.to_lowercase().contains(&q)
                    || s.metadata.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || s.metadata.category.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Get skills by category.
    pub fn by_category(&self, category: &str) -> Vec<&SkillManifest> {
        self.skills
            .values()
            .filter(|s| s.metadata.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// Get skills by tag.
    pub fn by_tag(&self, tag: &str) -> Vec<&SkillManifest> {
        let tag_lower = tag.to_lowercase();
        self.skills
            .values()
            .filter(|s| s.metadata.tags.iter().any(|t| t.to_lowercase() == tag_lower))
            .collect()
    }

    /// Get all unique categories.
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .skills
            .values()
            .map(|s| s.metadata.category.clone())
            .filter(|c| !c.is_empty())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Get all unique tags.
    pub fn tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .skills
            .values()
            .flat_map(|s| s.metadata.tags.clone())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }

    /// Load skills from a directory (scans for SKILL.md files).
    pub fn load_from_dir(&mut self, dir: &std::path::Path) -> Result<usize, String> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Read dir {}: {}", dir.display(), e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                if skill_file.exists() {
                    match SkillManifest::load(&skill_file) {
                        Ok(skill) => {
                            self.install(skill);
                            count += 1;
                        }
                        Err(e) => {
                            tracing::warn!("⚠ Failed to load skill from {}: {}", skill_file.display(), e);
                        }
                    }
                }
            } else if path.file_name().map_or(false, |n| n == "SKILL.md") {
                match SkillManifest::load(&path) {
                    Ok(skill) => {
                        self.install(skill);
                        count += 1;
                    }
                    Err(e) => {
                        tracing::warn!("⚠ Failed to load skill {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(count)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_skill(name: &str, category: &str, tags: Vec<&str>) -> SkillManifest {
        let raw = format!(
            "---\nname: {}\ndescription: A {} skill\ncategory: {}\ntags: [{}]\n---\nContent for {}.",
            name,
            name,
            category,
            tags.join(", "),
            name
        );
        SkillManifest::parse(&raw).unwrap()
    }

    #[test]
    fn test_registry_install_and_get() {
        let mut reg = SkillRegistry::new();
        reg.install(sample_skill("test-skill", "coding", vec!["rust"]));
        assert_eq!(reg.count(), 1);
        assert!(reg.get("test-skill").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_search() {
        let mut reg = SkillRegistry::new();
        reg.install(sample_skill("rust-expert", "coding", vec!["rust", "systems"]));
        reg.install(sample_skill("python-data", "data", vec!["python", "data"]));
        reg.install(sample_skill("web-scraper", "web", vec!["scraping", "python"]));

        let results = reg.search("python");
        assert_eq!(results.len(), 2);

        let results = reg.search("rust");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_registry_by_category() {
        let mut reg = SkillRegistry::new();
        reg.install(sample_skill("skill-a", "coding", vec![]));
        reg.install(sample_skill("skill-b", "coding", vec![]));
        reg.install(sample_skill("skill-c", "devops", vec![]));

        assert_eq!(reg.by_category("coding").len(), 2);
        assert_eq!(reg.by_category("devops").len(), 1);
        assert_eq!(reg.by_category("unknown").len(), 0);
    }

    #[test]
    fn test_registry_uninstall() {
        let mut reg = SkillRegistry::new();
        reg.install(sample_skill("temp", "misc", vec![]));
        assert!(reg.uninstall("temp"));
        assert!(!reg.uninstall("temp")); // already removed
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_registry_categories_and_tags() {
        let mut reg = SkillRegistry::new();
        reg.install(sample_skill("s1", "coding", vec!["rust", "web"]));
        reg.install(sample_skill("s2", "devops", vec!["docker", "web"]));

        let cats = reg.categories();
        assert!(cats.contains(&"coding".to_string()));
        assert!(cats.contains(&"devops".to_string()));

        let tags = reg.tags();
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"web".to_string()));
    }
}
