//! Skills Marketplace Web UI
//! 
//! Features:
//! - Skill browser với categories
//! - Search và filtering
//! - User reviews và ratings
//! - Installation flow

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceConfig {
    pub base_url: String,
    pub api_url: String,
    pub enable_ratings: bool,
    pub enable_reviews: bool,
    pub enable_installation: bool,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            base_url: "https://marketplace.bizclaw.ai".to_string(),
            api_url: "https://api.marketplace.bizclaw.ai".to_string(),
            enable_ratings: true,
            enable_reviews: true,
            enable_installation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStats {
    pub total_skills: u64,
    pub total_installs: u64,
    pub total_reviews: u64,
    pub avg_rating: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceFeatured {
    pub trending: Vec<Skill>,
    pub new: Vec<Skill>,
    pub top_rated: Vec<Skill>,
    pub by_category: Vec<CategoryFeatured>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryFeatured {
    pub category: String,
    pub skills: Vec<Skill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub short_description: String,
    pub version: String,
    pub author: Author,
    pub category: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub review_count: u32,
    pub verified: bool,
    pub featured: bool,
    pub min_bizclaw_version: String,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub install_count: u64,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub id: String,
    pub name: String,
    pub username: String,
    pub avatar: Option<String>,
    pub verified: bool,
    pub skill_count: u32,
    pub total_installs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub skill_id: String,
    pub user: Author,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub helpful_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewStats {
    pub total: u32,
    pub average: f32,
    pub distribution: RatingDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingDistribution {
    pub five_star: u32,
    pub four_star: u32,
    pub three_star: u32,
    pub two_star: u32,
    pub one_star: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub min_rating: Option<f32>,
    pub sort_by: SortOption,
    pub page: u32,
    pub per_page: u32,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            query: None,
            category: None,
            tags: None,
            author: None,
            min_rating: None,
            sort_by: SortOption::Popular,
            page: 1,
            per_page: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOption {
    Popular,
    Newest,
    Rating,
    Name,
    Downloads,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub skills: Vec<Skill>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub facets: SearchFacets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub categories: Vec<FacetValue>,
    pub tags: Vec<FacetValue>,
    pub authors: Vec<FacetValue>,
    pub ratings: Vec<FacetValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationRequest {
    pub skill_id: String,
    pub version: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationResult {
    pub success: bool,
    pub skill_id: String,
    pub version: String,
    pub message: String,
    pub warnings: Vec<String>,
}

pub struct MarketplaceService {
    config: MarketplaceConfig,
}

impl MarketplaceService {
    pub fn new(config: MarketplaceConfig) -> Self {
        Self { config }
    }

    pub fn get_featured(&self) -> MarketplaceFeatured {
        MarketplaceFeatured {
            trending: vec![],
            new: vec![],
            top_rated: vec![],
            by_category: vec![],
        }
    }

    pub fn search(&self, filters: SearchFilters) -> SearchResult {
        SearchResult {
            skills: vec![],
            total: 0,
            page: filters.page,
            per_page: filters.per_page,
            total_pages: 0,
            facets: SearchFacets {
                categories: vec![],
                tags: vec![],
                authors: vec![],
                ratings: vec![],
            },
        }
    }

    pub fn get_skill(&self, _slug: &str) -> Option<Skill> {
        None
    }

    pub fn get_reviews(&self, _skill_id: &str, _page: u32) -> Vec<Review> {
        vec![]
    }

    pub fn get_stats(&self) -> MarketplaceStats {
        MarketplaceStats {
            total_skills: 50,
            total_installs: 10000,
            total_reviews: 500,
            avg_rating: 4.2,
        }
    }

    pub fn get_categories(&self) -> Vec<Category> {
        vec![
            Category {
                id: "developer".to_string(),
                name: "Developer".to_string(),
                description: "Programming and development skills".to_string(),
                icon: "💻".to_string(),
                skill_count: 25,
            },
            Category {
                id: "business".to_string(),
                name: "Business".to_string(),
                description: "Business writing and communication".to_string(),
                icon: "💼".to_string(),
                skill_count: 15,
            },
            Category {
                id: "creative".to_string(),
                name: "Creative".to_string(),
                description: "Content creation and design".to_string(),
                icon: "🎨".to_string(),
                skill_count: 10,
            },
            Category {
                id: "data".to_string(),
                name: "Data".to_string(),
                description: "Data analysis and processing".to_string(),
                icon: "📊".to_string(),
                skill_count: 12,
            },
            Category {
                id: "automation".to_string(),
                name: "Automation".to_string(),
                description: "Workflow and process automation".to_string(),
                icon: "⚡".to_string(),
                skill_count: 8,
            },
            Category {
                id: "communication".to_string(),
                name: "Communication".to_string(),
                description: "Communication and collaboration".to_string(),
                icon: "💬".to_string(),
                skill_count: 6,
            },
            Category {
                id: "research".to_string(),
                name: "Research".to_string(),
                description: "Research and analysis".to_string(),
                icon: "🔬".to_string(),
                skill_count: 5,
            },
            Category {
                id: "education".to_string(),
                name: "Education".to_string(),
                description: "Teaching and learning".to_string(),
                icon: "📚".to_string(),
                skill_count: 4,
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub skill_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_config() {
        let config = MarketplaceConfig::default();
        assert!(config.enable_ratings);
        assert!(config.enable_reviews);
    }

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert_eq!(filters.page, 1);
        assert_eq!(filters.per_page, 20);
        assert!(matches!(filters.sort_by, SortOption::Popular));
    }

    #[test]
    fn test_categories() {
        let service = MarketplaceService::new(MarketplaceConfig::default());
        let categories = service.get_categories();
        assert_eq!(categories.len(), 8);
    }
}
