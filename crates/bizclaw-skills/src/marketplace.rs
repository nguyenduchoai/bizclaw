//! Skills marketplace — ClawHub-compatible skill discovery and installation.
//!
//! Integrates with:
//! - **ClawHub** (clawhub.ai) — the open-source OpenClaw skill registry
//! - **BizClaw Hub** (hub.bizclaw.vn) — BizClaw-specific skills
//!
//! Skills use the OpenClaw SKILL.md format with YAML frontmatter.

use serde::{Deserialize, Serialize};

/// A skill listing (compatible with both ClawHub and BizClaw Hub).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillListing {
    /// Skill slug (URL-safe identifier)
    #[serde(alias = "slug")]
    pub name: String,
    /// Display name
    #[serde(alias = "displayName", default)]
    pub display_name: String,
    /// Description (from SKILL.md frontmatter)
    #[serde(default)]
    pub description: String,
    /// Semver version
    #[serde(default)]
    pub version: String,
    /// Author handle or name
    #[serde(alias = "ownerHandle", default)]
    pub author: String,
    /// Category (AI, Business, Channel, etc.)
    #[serde(default)]
    pub category: String,
    /// Business category (e.g., "sales", "marketing")
    #[serde(default)]
    pub business_category: String,
    /// Business roles that would use this skill
    #[serde(default)]
    pub business_roles: Vec<String>,
    /// Industry tags
    #[serde(default)]
    pub industry: Vec<String>,
    /// Pain points addressed
    #[serde(default)]
    pub pain_points: Vec<String>,
    /// Tags for search/filter
    #[serde(default)]
    pub tags: Vec<String>,
    /// Emoji icon
    #[serde(alias = "emoji", default)]
    pub icon: String,
    /// Download count
    #[serde(alias = "installsAllTime", default)]
    pub downloads: u64,
    /// Rating (0-5)
    #[serde(default)]
    pub rating: f32,
    /// Source URL
    #[serde(alias = "homepage", default)]
    pub url: String,
    /// Install source: "clawhub", "bizclaw", or "local"
    #[serde(default)]
    pub source: String,
}

/// ClawHub API response for skill list.
#[derive(Debug, Deserialize)]
struct ClawHubSkillsResponse {
    #[serde(default)]
    skills: Vec<SkillListing>,
}

/// Registry source configuration.
#[derive(Debug, Clone)]
pub struct RegistrySource {
    pub name: String,
    pub api_url: String,
    pub enabled: bool,
}

/// Skills marketplace client — supports multiple registries.
pub struct SkillMarketplace {
    /// Registry sources (ClawHub, BizClaw Hub, etc.)
    sources: Vec<RegistrySource>,
    /// Cached listings from all sources.
    cache: Vec<SkillListing>,
}

impl SkillMarketplace {
    /// Create a new marketplace with default registries.
    pub fn new() -> Self {
        Self {
            sources: vec![
                RegistrySource {
                    name: "clawhub".into(),
                    api_url: "https://clawhub.ai/api/v1".into(),
                    enabled: true,
                },
                RegistrySource {
                    name: "bizclaw".into(),
                    api_url: "https://hub.bizclaw.vn/api/v1".into(),
                    enabled: true,
                },
            ],
            cache: Vec::new(),
        }
    }

    /// Create with custom registry URL.
    pub fn with_registry(name: &str, api_url: &str) -> Self {
        Self {
            sources: vec![RegistrySource {
                name: name.into(),
                api_url: api_url.into(),
                enabled: true,
            }],
            cache: Vec::new(),
        }
    }

    /// Add a registry source.
    pub fn add_source(&mut self, source: RegistrySource) {
        self.sources.push(source);
    }

    /// Get the primary API URL (first enabled source).
    pub fn base_url(&self) -> &str {
        self.sources
            .iter()
            .find(|s| s.enabled)
            .map(|s| s.api_url.as_str())
            .unwrap_or("https://clawhub.ai/api/v1")
    }

    // ═══════════════════════════════════════════════════════════
    // Discovery (ClawHub API: /api/v1/skills, /api/v1/search)
    // ═══════════════════════════════════════════════════════════

    /// Fetch skills from all enabled registries.
    /// Maps to ClawHub: `GET /api/v1/skills?limit=N&sort=S`
    pub async fn fetch_from_registries(
        &mut self,
        limit: u32,
        sort: &str,
    ) -> Result<Vec<SkillListing>, String> {
        let client = reqwest::Client::new();
        let mut all_skills = Vec::new();

        for source in &self.sources {
            if !source.enabled {
                continue;
            }
            let url = format!("{}/skills?limit={}&sort={}", source.api_url, limit, sort);
            tracing::debug!("🔍 Fetching skills from {} ({})", source.name, url);

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.text().await {
                        // Try parsing as array or as { skills: [...] }
                        if let Ok(mut skills) = serde_json::from_str::<Vec<SkillListing>>(&body) {
                            for s in &mut skills {
                                s.source = source.name.clone();
                            }
                            tracing::info!("📦 {} skills from {}", skills.len(), source.name);
                            all_skills.extend(skills);
                        } else if let Ok(resp) =
                            serde_json::from_str::<ClawHubSkillsResponse>(&body)
                        {
                            let mut skills = resp.skills;
                            for s in &mut skills {
                                s.source = source.name.clone();
                            }
                            tracing::info!("📦 {} skills from {}", skills.len(), source.name);
                            all_skills.extend(skills);
                        }
                    }
                }
                Ok(resp) => {
                    tracing::warn!("⚠️ {} returned HTTP {}", source.name, resp.status());
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to reach {}: {}", source.name, e);
                }
            }
        }

        self.cache = all_skills.clone();
        Ok(all_skills)
    }

    /// Search skills using ClawHub vector search.
    /// Maps to ClawHub: `GET /api/v1/search?q=QUERY`
    pub async fn search_remote(&self, query: &str) -> Result<Vec<SkillListing>, String> {
        let client = reqwest::Client::new();
        let mut results = Vec::new();

        for source in &self.sources {
            if !source.enabled {
                continue;
            }
            let url = format!("{}/search?q={}", source.api_url, urlencoding::encode(query));
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(mut skills) = resp.json::<Vec<SkillListing>>().await {
                        for s in &mut skills {
                            s.source = source.name.clone();
                        }
                        results.extend(skills);
                    }
                }
                _ => {}
            }
        }
        Ok(results)
    }

    /// Inspect a skill (get full metadata without installing).
    /// Maps to ClawHub: `GET /api/v1/skills/{slug}`
    pub async fn inspect(&self, slug: &str) -> Result<SkillListing, String> {
        let client = reqwest::Client::new();

        for source in &self.sources {
            if !source.enabled {
                continue;
            }
            let url = format!("{}/skills/{}", source.api_url, slug);
            if let Ok(resp) = client.get(&url).send().await
                && resp.status().is_success()
                && let Ok(skill) = resp.json::<SkillListing>().await
            {
                return Ok(skill);
            }
        }
        Err(format!("Skill '{}' not found in any registry", slug))
    }

    // ═══════════════════════════════════════════════════════════
    // Installation (ClawHub API: /api/v1/download)
    // ═══════════════════════════════════════════════════════════

    fn is_valid_slug(slug: &str) -> bool {
        !slug.is_empty()
            && slug.len() <= 128
            && slug
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    /// Install a skill from a registry.
    /// Maps to ClawHub: `GET /api/v1/download?slug=SLUG&version=VERSION`
    pub async fn install(
        &mut self,
        slug: &str,
        install_dir: &std::path::Path,
    ) -> Result<SkillListing, String> {
        if !Self::is_valid_slug(slug) {
            return Err(format!(
                "Invalid slug '{}': must be alphanumeric, hyphen, or underscore (max 128 chars)",
                slug
            ));
        }

        let client = reqwest::Client::new();
        tracing::info!("📦 Installing skill '{}'...", slug);

        for source in &self.sources {
            if !source.enabled {
                continue;
            }

            // 1. Get skill metadata
            let meta_url = format!("{}/skills/{}", source.api_url, slug);
            let listing = match client.get(&meta_url).send().await {
                Ok(r) if r.status().is_success() => match r.json::<SkillListing>().await {
                    Ok(mut s) => {
                        s.source = source.name.clone();
                        s
                    }
                    Err(_) => continue,
                },
                _ => continue,
            };

            // 2. Download skill zip
            let download_url = format!(
                "{}/download?slug={}&version={}",
                source.api_url, slug, listing.version
            );
            let archive = match client.get(&download_url).send().await {
                Ok(r) if r.status().is_success() => r
                    .bytes()
                    .await
                    .map_err(|e| format!("Download failed: {}", e))?,
                Ok(r) => {
                    tracing::warn!("Download returned HTTP {}", r.status());
                    continue;
                }
                Err(e) => {
                    tracing::warn!("Download failed: {}", e);
                    continue;
                }
            };

            // 3. Extract to install_dir/<slug>/
            let skill_dir = install_dir.join(slug);
            std::fs::create_dir_all(&skill_dir)
                .map_err(|e| format!("Failed to create dir: {}", e))?;

            // Write skill archive (zip from ClawHub)
            let archive_path = skill_dir.join("skill-archive.zip");
            std::fs::write(&archive_path, &archive)
                .map_err(|e| format!("Failed to write: {}", e))?;

            // 4. Write origin metadata (ClawHub convention)
            let origin_dir = skill_dir.join(".clawhub");
            std::fs::create_dir_all(&origin_dir)
                .map_err(|e| format!("Failed to create origin dir: {}", e))?;
            let origin = serde_json::json!({
                "slug": slug,
                "version": listing.version,
                "source": source.name,
                "registry": source.api_url,
                "installed_at": chrono::Utc::now().to_rfc3339(),
            });
            std::fs::write(
                origin_dir.join("origin.json"),
                serde_json::to_string_pretty(&origin)
                    .map_err(|e| format!("Failed to serialize origin: {}", e))?,
            )
            .map_err(|e| format!("Failed to write origin.json: {}", e))?;

            // 5. Update cache
            if !self.cache.iter().any(|s| s.name == slug) {
                self.cache.push(listing.clone());
            }

            tracing::info!(
                "✅ Installed '{}' v{} from {}",
                slug,
                listing.version,
                source.name
            );
            return Ok(listing);
        }

        Err(format!("Skill '{}' not found in any registry", slug))
    }

    /// Uninstall a skill.
    pub fn uninstall(&mut self, slug: &str, install_dir: &std::path::Path) -> Result<(), String> {
        if !Self::is_valid_slug(slug) {
            return Err(format!(
                "Invalid slug '{}': must be alphanumeric, hyphen, or underscore",
                slug
            ));
        }
        let skill_dir = install_dir.join(slug);
        if skill_dir.exists() {
            std::fs::remove_dir_all(&skill_dir).map_err(|e| format!("Failed to remove: {}", e))?;
        }
        self.cache.retain(|s| s.name != slug);
        tracing::info!("🗑️ Uninstalled skill '{}'", slug);
        Ok(())
    }

    /// Check for updates across all installed skills.
    /// Compares local versions with registry versions.
    pub async fn check_updates(
        &self,
        install_dir: &std::path::Path,
    ) -> Vec<(String, String, String)> {
        let client = reqwest::Client::new();
        let mut updates = Vec::new();

        for skill in &self.cache {
            // Read local origin.json
            let origin_path = install_dir.join(&skill.name).join(".clawhub/origin.json");
            let local_version = if origin_path.exists() {
                std::fs::read_to_string(&origin_path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .and_then(|v| v["version"].as_str().map(String::from))
                    .unwrap_or_else(|| skill.version.clone())
            } else {
                skill.version.clone()
            };

            // Check remote version
            for source in &self.sources {
                if !source.enabled {
                    continue;
                }
                let url = format!("{}/skills/{}", source.api_url, skill.name);
                if let Ok(resp) = client.get(&url).send().await
                    && let Ok(remote) = resp.json::<SkillListing>().await
                {
                    if remote.version != local_version {
                        updates.push((skill.name.clone(), local_version.clone(), remote.version));
                    }
                    break; // Found in this source, don't check others
                }
            }
        }

        updates
    }

    // ═══════════════════════════════════════════════════════════
    // Local search/filter (cache-based)
    // ═══════════════════════════════════════════════════════════

    /// Search cached skills locally.
    pub fn search(&self, query: &str) -> Vec<&SkillListing> {
        let q = query.to_lowercase();
        self.cache
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&q)
                    || s.description.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    /// List all cached skills.
    pub fn list(&self) -> &[SkillListing] {
        &self.cache
    }

    /// Get by category.
    pub fn by_category(&self, category: &str) -> Vec<&SkillListing> {
        self.cache
            .iter()
            .filter(|s| s.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// Filter by source registry.
    pub fn by_source(&self, source: &str) -> Vec<&SkillListing> {
        self.cache.iter().filter(|s| s.source == source).collect()
    }

    /// Add a listing to the cache (for built-in/offline mode).
    pub fn add_listing(&mut self, listing: SkillListing) {
        self.cache.push(listing);
    }

    /// Count cached listings.
    pub fn count(&self) -> usize {
        self.cache.len()
    }

    /// Sort by downloads (most popular first).
    pub fn sort_by_popularity(&mut self) {
        self.cache.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    }

    /// Sort by rating (highest first).
    pub fn sort_by_rating(&mut self) {
        self.cache.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Filter by business role.
    pub fn by_business_role(&self, role: &str) -> Vec<&SkillListing> {
        let role_lower = role.to_lowercase();
        self.cache
            .iter()
            .filter(|s| {
                s.business_roles
                    .iter()
                    .any(|r| r.to_lowercase().contains(&role_lower))
                    || s.business_category.to_lowercase().contains(&role_lower)
            })
            .collect()
    }

    /// Filter by industry.
    pub fn by_industry(&self, industry: &str) -> Vec<&SkillListing> {
        let industry_lower = industry.to_lowercase();
        self.cache
            .iter()
            .filter(|s| {
                s.industry
                    .iter()
                    .any(|i| i.to_lowercase().contains(&industry_lower))
            })
            .collect()
    }

    // ═══════════════════════════════════════════════════════════
    // LLM-based semantic search (Harrier embeddings)
    // ═══════════════════════════════════════════════════════════

    /// Find skills matching a task description using semantic search.
    /// Uses Harrier embeddings for vector similarity search.
    pub async fn find_skills_for_task(
        &self,
        task_description: &str,
        embed_api_url: Option<String>,
        embed_api_key: Option<String>,
    ) -> Vec<(SkillListing, f32)> {
        use crate::harrier;

        if self.cache.is_empty() {
            return Vec::new();
        }

        let task_embedding = match harrier::execute_local_harrier_embed(
            task_description,
            "retrieval_query",
            embed_api_url.clone(),
            embed_api_key.clone(),
        )
        .await
        {
            Ok(emb) => emb,
            Err(e) => {
                tracing::warn!("Failed to get embeddings for task search: {}", e);
                return Vec::new();
            }
        };

        let mut scored: Vec<(SkillListing, f32)> = self
            .cache
            .iter()
            .map(|s| {
                let skill_text = format!(
                    "{} {} {} {:?} {:?}",
                    s.name, s.display_name, s.description, s.tags, s.business_category
                );
                let sim = cosine_similarity(&task_embedding, &skill_text);
                (s.clone(), sim)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(10);
        scored
    }
}

fn cosine_similarity(a: &[f32], text_b: &str) -> f32 {
    let b = simple_embedding(text_b, a.len());
    if b.is_none() {
        return 0.0;
    }
    let b = b.unwrap();

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}

#[deprecated(
    since = "1.1.7",
    note = "simple_embedding() uses hash-based vectors which produce meaningless cosine similarity. \
            find_skills_for_task() is currently a no-op for semantic search. \
            TODO: Replace with real embeddings from Ollama/OpenAI API, or remove this function."
)]
fn simple_embedding(text: &str, dim: usize) -> Option<Vec<f32>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let h = hasher.finish();

    let mut vec = Vec::with_capacity(dim);
    for i in 0..dim {
        let val = (((h.wrapping_mul(i as u64).wrapping_add(i as u64)) % 1000) as f32) / 1000.0;
        vec.push(val);
    }
    let len = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if len > 0.0 {
        vec.iter_mut().for_each(|x| *x /= len);
    }
    Some(vec)
}

impl Default for SkillMarketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_listing(name: &str, category: &str, downloads: u64) -> SkillListing {
        SkillListing {
            name: name.to_string(),
            display_name: name.replace('-', " "),
            description: format!("A {} skill", name),
            version: "1.0.0".to_string(),
            author: "BizClaw".to_string(),
            category: category.to_string(),
            business_category: String::new(),
            business_roles: Vec::new(),
            industry: Vec::new(),
            pain_points: Vec::new(),
            tags: vec![category.to_string()],
            icon: "📦".to_string(),
            downloads,
            rating: 4.5,
            url: format!("https://clawhub.ai/skills/{}", name),
            source: "clawhub".to_string(),
        }
    }

    #[test]
    fn test_marketplace_search() {
        let mut mp = SkillMarketplace::with_registry("test", "https://test");
        mp.add_listing(sample_listing("rust-dev", "coding", 100));
        mp.add_listing(sample_listing("python-ml", "data", 200));
        mp.add_listing(sample_listing("devops-k8s", "devops", 50));

        assert_eq!(mp.search("rust").len(), 1);
        assert_eq!(mp.search("coding").len(), 1);
        assert_eq!(mp.count(), 3);
    }

    #[test]
    fn test_marketplace_sort() {
        let mut mp = SkillMarketplace::with_registry("test", "https://test");
        mp.add_listing(sample_listing("a", "x", 10));
        mp.add_listing(sample_listing("b", "x", 100));
        mp.add_listing(sample_listing("c", "x", 50));

        mp.sort_by_popularity();
        assert_eq!(mp.list()[0].name, "b");
        assert_eq!(mp.list()[2].name, "a");
    }

    #[test]
    fn test_default_registries() {
        let mp = SkillMarketplace::new();
        assert_eq!(mp.sources.len(), 2);
        assert_eq!(mp.sources[0].name, "clawhub");
        assert_eq!(mp.sources[1].name, "bizclaw");
    }

    #[test]
    fn test_by_source() {
        let mut mp = SkillMarketplace::with_registry("test", "https://test");
        let mut s1 = sample_listing("a", "x", 10);
        s1.source = "clawhub".into();
        let mut s2 = sample_listing("b", "x", 10);
        s2.source = "bizclaw".into();
        mp.add_listing(s1);
        mp.add_listing(s2);

        assert_eq!(mp.by_source("clawhub").len(), 1);
        assert_eq!(mp.by_source("bizclaw").len(), 1);
    }
}
