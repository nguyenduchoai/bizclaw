//! Skills marketplace — remote skill discovery and installation.

use serde::{Deserialize, Serialize};

/// A skill listing from the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillListing {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub icon: String,
    pub downloads: u64,
    pub rating: f32,
    pub url: String,
}

/// Skills marketplace client.
pub struct SkillMarketplace {
    /// Base URL for the marketplace API.
    base_url: String,
    /// Cached listings.
    cache: Vec<SkillListing>,
}

impl SkillMarketplace {
    /// Create a new marketplace client.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            cache: Vec::new(),
        }
    }

    /// Default marketplace (BizClaw Hub).
    pub fn default_hub() -> Self {
        Self::new("https://hub.bizclaw.vn/api/v1/skills")
    }

    /// Search the marketplace (local cache for now).
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

    /// Get the marketplace base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
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
}

impl Default for SkillMarketplace {
    fn default() -> Self {
        Self::default_hub()
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
            tags: vec![category.to_string()],
            icon: "📦".to_string(),
            downloads,
            rating: 4.5,
            url: format!("https://hub.bizclaw.vn/skills/{}", name),
        }
    }

    #[test]
    fn test_marketplace_search() {
        let mut mp = SkillMarketplace::new("https://test");
        mp.add_listing(sample_listing("rust-dev", "coding", 100));
        mp.add_listing(sample_listing("python-ml", "data", 200));
        mp.add_listing(sample_listing("devops-k8s", "devops", 50));

        assert_eq!(mp.search("rust").len(), 1);
        assert_eq!(mp.search("coding").len(), 1);
        assert_eq!(mp.count(), 3);
    }

    #[test]
    fn test_marketplace_sort() {
        let mut mp = SkillMarketplace::new("https://test");
        mp.add_listing(sample_listing("a", "x", 10));
        mp.add_listing(sample_listing("b", "x", 100));
        mp.add_listing(sample_listing("c", "x", 50));

        mp.sort_by_popularity();
        assert_eq!(mp.list()[0].name, "b");
        assert_eq!(mp.list()[2].name, "a");
    }
}
