//! Search result types and filters — lightweight BizClaw style.

use serde::{Deserialize, Serialize};

/// A single search result from the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document name.
    pub doc_name: String,
    /// Chunk index within the document.
    pub chunk_idx: usize,
    /// The matching text content.
    pub content: String,
    /// Relevance score (higher = more relevant after normalization).
    pub score: f64,
    /// Document metadata (populated when available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mimetype: Option<String>,
    /// Who uploaded/owns this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

impl SearchResult {
    /// Format as context for the Agent system prompt.
    pub fn as_context(&self) -> String {
        format!("[📄 {}] {}", self.doc_name, self.content)
    }
}

/// Search filters — narrow results by document metadata.
/// All fields are optional. Multiple values = OR within a field, AND across fields.
///
/// # Example
/// ```
/// use bizclaw_knowledge::search::SearchFilter;
/// let filter = SearchFilter {
///     doc_names: Some(vec!["policy.md".into(), "rules.md".into()]),
///     mimetypes: Some(vec!["text/markdown".into()]),
///     owners: None,
///     score_threshold: Some(0.1),
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilter {
    /// Filter by document names (exact match).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_names: Option<Vec<String>>,

    /// Filter by MIME types (e.g., "application/pdf", "text/markdown").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mimetypes: Option<Vec<String>>,

    /// Filter by owner/uploader ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<String>>,

    /// Minimum score threshold (skip low-quality results).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f64>,
}

impl SearchFilter {
    /// Check if any filters are active.
    pub fn is_empty(&self) -> bool {
        self.doc_names.is_none()
            && self.mimetypes.is_none()
            && self.owners.is_none()
            && self.score_threshold.is_none()
    }

    /// Check if a search result passes all active filters.
    pub fn matches(&self, result: &SearchResult) -> bool {
        // Check doc_names filter
        if let Some(names) = &self.doc_names
            && !names.iter().any(|n| n == &result.doc_name)
        {
            return false;
        }

        // Check mimetypes filter
        if let Some(types) = &self.mimetypes {
            match &result.mimetype {
                Some(mt) => {
                    if !types.iter().any(|t| t == mt) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Check owners filter
        if let Some(owners) = &self.owners {
            match &result.owner {
                Some(o) => {
                    if !owners.iter().any(|ow| ow == o) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Check score threshold
        if let Some(threshold) = self.score_threshold
            && result.score < threshold
        {
            return false;
        }

        true
    }

    /// Build a SQL WHERE clause fragment for document-level filters.
    /// Returns (clause, params) where clause is like "AND d.mimetype IN (?1, ?2)".
    /// This is used for pre-filtering at the SQL level (more efficient than post-filter).
    pub fn to_sql_conditions(&self) -> (String, Vec<String>) {
        let mut clauses = Vec::new();
        let mut params = Vec::new();

        if let Some(names) = &self.doc_names
            && !names.is_empty()
        {
            let placeholders: Vec<String> = (0..names.len()).map(|_| "?".to_string()).collect();
            clauses.push(format!("d.name IN ({})", placeholders.join(",")));
            params.extend(names.clone());
        }

        if let Some(types) = &self.mimetypes
            && !types.is_empty()
        {
            let placeholders: Vec<String> = (0..types.len()).map(|_| "?".to_string()).collect();
            clauses.push(format!("d.mimetype IN ({})", placeholders.join(",")));
            params.extend(types.clone());
        }

        if let Some(owners) = &self.owners
            && !owners.is_empty()
        {
            let placeholders: Vec<String> = (0..owners.len()).map(|_| "?".to_string()).collect();
            clauses.push(format!("d.owner IN ({})", placeholders.join(",")));
            params.extend(owners.clone());
        }

        let clause = if clauses.is_empty() {
            String::new()
        } else {
            format!(" AND {}", clauses.join(" AND "))
        };

        (clause, params)
    }
}

/// Format multiple search results as Agent context.
pub fn format_knowledge_context(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut ctx = String::from("\n--- Knowledge Base Context ---\n");
    for (i, r) in results.iter().enumerate() {
        ctx.push_str(&format!("[{}] {}\n\n", i + 1, r.as_context()));
    }
    ctx.push_str("--- End Knowledge ---\n");
    ctx
}

/// Search telemetry for RAG observability (Goclaw-inspired).
///
/// Track hit/miss rates, search latency, and boost effectiveness
/// to monitor knowledge base health in production.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchTelemetry {
    /// Number of query terms extracted.
    pub query_terms: usize,
    /// Results from keyword search (pre-boost).
    pub keyword_results: usize,
    /// Final results after boosting and truncation.
    pub final_results: usize,
    /// Whether TF-IDF boost was applied.
    pub boost_applied: bool,
    /// Total search latency in milliseconds.
    pub search_ms: u64,
    /// Whether the search produced any results (hit=true, miss=false).
    pub hit: bool,
}

impl SearchTelemetry {
    /// Create an empty telemetry record (for when search is skipped).
    pub fn empty() -> Self {
        Self {
            query_terms: 0,
            keyword_results: 0,
            final_results: 0,
            boost_applied: false,
            search_ms: 0,
            hit: false,
        }
    }

    /// Human-readable summary for logging.
    pub fn summary(&self) -> String {
        format!(
            "{} | {} terms | {}/{} results | {}ms",
            if self.hit { "HIT" } else { "MISS" },
            self.query_terms,
            self.final_results,
            self.keyword_results,
            self.search_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(
        name: &str,
        mime: Option<&str>,
        owner: Option<&str>,
        score: f64,
    ) -> SearchResult {
        SearchResult {
            doc_name: name.to_string(),
            chunk_idx: 0,
            content: "test".to_string(),
            score,
            mimetype: mime.map(String::from),
            owner: owner.map(String::from),
        }
    }

    #[test]
    fn test_empty_filter_matches_all() {
        let filter = SearchFilter::default();
        assert!(filter.is_empty());
        assert!(filter.matches(&make_result("any.md", None, None, 0.5)));
    }

    #[test]
    fn test_doc_name_filter() {
        let filter = SearchFilter {
            doc_names: Some(vec!["policy.md".into()]),
            ..Default::default()
        };
        assert!(filter.matches(&make_result("policy.md", None, None, 0.5)));
        assert!(!filter.matches(&make_result("other.md", None, None, 0.5)));
    }

    #[test]
    fn test_mimetype_filter() {
        let filter = SearchFilter {
            mimetypes: Some(vec!["application/pdf".into()]),
            ..Default::default()
        };
        assert!(filter.matches(&make_result("doc.pdf", Some("application/pdf"), None, 0.5)));
        assert!(!filter.matches(&make_result("doc.md", Some("text/markdown"), None, 0.5)));
        assert!(!filter.matches(&make_result("doc.md", None, None, 0.5))); // no mimetype
    }

    #[test]
    fn test_score_threshold() {
        let filter = SearchFilter {
            score_threshold: Some(0.3),
            ..Default::default()
        };
        assert!(filter.matches(&make_result("doc.md", None, None, 0.5)));
        assert!(!filter.matches(&make_result("doc.md", None, None, 0.1)));
    }

    #[test]
    fn test_combined_filters() {
        let filter = SearchFilter {
            doc_names: Some(vec!["policy.md".into(), "rules.md".into()]),
            score_threshold: Some(0.2),
            ..Default::default()
        };
        assert!(filter.matches(&make_result("policy.md", None, None, 0.5)));
        assert!(filter.matches(&make_result("rules.md", None, None, 0.3)));
        assert!(!filter.matches(&make_result("other.md", None, None, 0.5))); // wrong name
        assert!(!filter.matches(&make_result("policy.md", None, None, 0.1))); // low score
    }

    #[test]
    fn test_sql_conditions() {
        let filter = SearchFilter {
            doc_names: Some(vec!["a.md".into(), "b.md".into()]),
            mimetypes: Some(vec!["text/markdown".into()]),
            ..Default::default()
        };
        let (clause, params) = filter.to_sql_conditions();
        assert!(clause.contains("d.name IN"));
        assert!(clause.contains("d.mimetype IN"));
        assert_eq!(params.len(), 3);
    }
}
