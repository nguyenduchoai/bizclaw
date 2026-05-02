//! Tantivy index implementation

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tantivy::collector::{FacetCounts, TopDocs};
use tantivy::query::{FuzzyTermQuery, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::query::{AutocompleteResult, FacetResult, SearchOptions, SearchQuery, SearchResponse, SearchResult, Suggestion};
use crate::schema::{create_schema, IndexedDocument};

pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Arc<RwLock<IndexWriter>>,
    schema: Schema,
    id_field: Field,
    doc_type_field: Field,
    title_field: Field,
    content_field: Field,
    tags_field: Field,
    author_field: Field,
    source_field: Field,
    url_field: Field,
    created_at_field: Field,
    updated_at_field: Field,
    session_id_field: Field,
    access_count_field: Field,
    relevance_score_field: Field,
}

impl SearchIndex {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        // Ensure directory exists
        std::fs::create_dir_all(path)?;
        
        let schema = create_schema();
        
        // Create or open index
        let index = if path.join("meta.json").exists() {
            Index::open_in_dir(path)?
        } else {
            Index::create_in_dir(path, schema.clone())?
        };
        
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        
        let writer = index.writer(50_000_000)?; // 50MB buffer
        
        let id_field = schema.get_field("id")?;
        let doc_type_field = schema.get_field("doc_type")?;
        let title_field = schema.get_field("title")?;
        let content_field = schema.get_field("content")?;
        let tags_field = schema.get_field("tags")?;
        let author_field = schema.get_field("author")?;
        let source_field = schema.get_field("source")?;
        let url_field = schema.get_field("url")?;
        let created_at_field = schema.get_field("created_at")?;
        let updated_at_field = schema.get_field("updated_at")?;
        let session_id_field = schema.get_field("session_id")?;
        let access_count_field = schema.get_field("access_count")?;
        let relevance_score_field = schema.get_field("relevance_score")?;
        
        info!("SearchIndex initialized at {:?}", path);
        
        Ok(Self {
            index,
            reader,
            writer: Arc::new(RwLock::new(writer)),
            schema,
            id_field,
            doc_type_field,
            title_field,
            content_field,
            tags_field,
            author_field,
            source_field,
            url_field,
            created_at_field,
            updated_at_field,
            session_id_field,
            access_count_field,
            relevance_score_field,
        })
    }

    pub async fn add_document(&self, doc: IndexedDocument) -> Result<()> {
        let writer = self.writer.write().await;
        
        let mut tantivy_doc = TantivyDocument::default();
        tantivy_doc.add_text(self.id_field, &doc.id);
        tantivy_doc.add_text(self.doc_type_field, doc.doc_type.as_str());
        
        if let Some(title) = &doc.title {
            tantivy_doc.add_text(self.title_field, title);
        }
        
        tantivy_doc.add_text(self.content_field, &doc.content);
        
        for tag in &doc.tags {
            tantivy_doc.add_text(self.tags_field, tag);
        }
        
        if let Some(author) = &doc.author {
            tantivy_doc.add_text(self.author_field, author);
        }
        
        if let Some(source) = &doc.source {
            tantivy_doc.add_text(self.source_field, source);
        }
        
        if let Some(url) = &doc.url {
            tantivy_doc.add_text(self.url_field, url);
        }
        
        tantivy_doc.add_i64(self.created_at_field, doc.created_at);
        tantivy_doc.add_i64(self.updated_at_field, doc.updated_at);
        
        if let Some(session_id) = &doc.session_id {
            tantivy_doc.add_text(self.session_id_field, session_id);
        }
        
        tantivy_doc.add_i64(self.access_count_field, doc.access_count);
        tantivy_doc.add_f64(self.relevance_score_field, 0.0);
        
        writer.add_document(tantivy_doc)?;
        
        debug!("ADDED document: {}", doc.id);
        Ok(())
    }

    pub async fn add_documents(&self, docs: Vec<IndexedDocument>) -> Result<()> {
        for doc in docs {
            self.add_document(doc).await?;
        }
        Ok(())
    }

    pub async fn update_document(&self, doc: IndexedDocument) -> Result<()> {
        // Delete old version first
        self.delete_document(&doc.id).await?;
        // Add new version
        self.add_document(doc).await?;
        Ok(())
    }

    pub async fn delete_document(&self, id: &str) -> Result<bool> {
        let writer = self.writer.write().await;
        
        let term = tantivy::Term::from_field_text(self.id_field, id);
        let deleted = writer.delete_term(term);
        
        debug!("DELETED document: {} (found={})", id, deleted);
        Ok(deleted > 0)
    }

    pub async fn delete_by_query(&self, query: &str) -> Result<usize> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.content_field, self.title_field, self.tags_field],
        );
        
        let parsed = query_parser.parse_query(query)?;
        let docs = searcher.search(&parsed, &tantivy::collector::TopDocs::with_limit(1000))?;
        
        let writer = self.writer.write().await;
        let mut count = 0;
        
        for (_score, doc_address) in docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id_value) = doc.get_first(self.id_field) {
                if let Some(id_str) = id_value.as_str() {
                    let term = tantivy::Term::from_field_text(self.id_field, id_str);
                    if writer.delete_term(term) > 0 {
                        count += 1;
                    }
                }
            }
        }
        
        info!("DELETED {} documents matching query: {}", count, query);
        Ok(count)
    }

    pub async fn search(
        &self,
        query: SearchQuery,
        options: SearchOptions,
    ) -> Result<SearchResponse> {
        let start = Instant::now();
        let searcher = self.reader.searcher();
        
        // Build query parser
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.title_field, self.content_field, self.tags_field],
        );
        
        // Parse the query string
        let parsed_query = query_parser.parse_query(&query.query_string)?;
        
        // Execute search
        let offset = query.offset();
        let limit = query.per_page;
        
        let top_docs = searcher.search(
            &parsed_query,
            &TopDocs::with_limit(limit).and_offset(offset),
        )?;
        
        let total = searcher.search(&parsed_query, &TopDocs::with_limit(10000))?.len();
        
        let mut results = Vec::new();
        
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            
            let id = get_text(&doc, self.id_field)?;
            let doc_type = get_text(&doc, self.doc_type_field)?;
            let title = get_optional_text(&doc, self.title_field);
            let content = get_text(&doc, self.content_field)?;
            let tags = get_all_text(&doc, self.tags_field);
            let created_at = get_i64(&doc, self.created_at_field)?;
            let updated_at = get_i64(&doc, self.updated_at_field)?;
            let session_id = get_optional_text(&doc, self.session_id_field);
            let access_count = get_i64(&doc, self.access_count_field)?;
            
            // Generate highlights
            let highlights = if options.include_snippets {
                generate_highlights(&content, &query.query_string, options.highlight_fragments)
            } else {
                Vec::new()
            };
            
            results.push(SearchResult {
                id,
                doc_type,
                title,
                content,
                tags,
                score,
                highlights,
                created_at,
                updated_at,
                session_id,
                access_count,
            });
        }
        
        let took_ms = start.elapsed().as_millis() as u64;
        
        let mut response = SearchResponse::new(query.query_string, results, total, took_ms);
        response = response.with_pagination(query.page, query.per_page);
        
        // Add facets if requested
        let facets = self.get_facets().await?;
        response = response.with_facets(facets);
        
        Ok(response)
    }

    pub async fn search_fuzzy(
        &self,
        term: &str,
        distance: usize,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        
        let term_query = FuzzyTermQuery::new(
            tantivy::Term::from_field_text(self.content_field, term),
            distance as u8,
            true,
        );
        
        let top_docs = searcher.search(&term_query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            
            results.push(SearchResult {
                id: get_text(&doc, self.id_field)?,
                doc_type: get_text(&doc, self.doc_type_field)?,
                title: get_optional_text(&doc, self.title_field),
                content: get_text(&doc, self.content_field)?,
                tags: get_all_text(&doc, self.tags_field),
                score,
                highlights: Vec::new(),
                created_at: get_i64(&doc, self.created_at_field)?,
                updated_at: get_i64(&doc, self.updated_at_field)?,
                session_id: get_optional_text(&doc, self.session_id_field),
                access_count: get_i64(&doc, self.access_count_field)?,
            });
        }
        
        Ok(results)
    }

    pub async fn autocomplete(&self, prefix: &str, limit: usize) -> Result<AutocompleteResult> {
        let searcher = self.reader.searcher();
        
        let term = tantivy::Term::from_field_text(self.content_field, prefix);
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);
        
        let top_docs = searcher.search(&term_query, &TopDocs::with_limit(limit))?;
        
        let mut suggestions: std::collections::HashMap<String, (f32, usize)> = 
            std::collections::HashMap::new();
        
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            let content = get_text(&doc, self.content_field)?;
            
            // Extract words starting with prefix
            for word in content.split_whitespace() {
                let word_lower = word.to_lowercase();
                if word_lower.starts_with(&prefix.to_lowercase()) && word.len() > prefix.len() {
                    let entry = suggestions.entry(word_lower).or_insert((0.0, 0));
                    entry.0 += score;
                    entry.1 += 1;
                }
            }
        }
        
        let mut results: Vec<Suggestion> = suggestions
            .into_iter()
            .map(|(text, (score, count))| Suggestion { text, score, doc_count: count })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        
        Ok(AutocompleteResult { suggestions: results })
    }

    pub async fn get_facets(&self) -> Result<Vec<FacetResult>> {
        Ok(Vec::new())
    }

    pub async fn commit(&self) -> Result<()> {
        let mut writer = self.writer.write().await;
        writer.commit()?;
        info!("Index committed successfully");
        Ok(())
    }

    pub async fn count(&self) -> Result<usize> {
        let searcher = self.reader.searcher();
        Ok(searcher.search(&tantivy::query::AllQuery, &TopDocs::with_limit(0))?.len())
    }

    pub async fn get(&self, id: &str) -> Result<Option<IndexedDocument>> {
        let searcher = self.reader.searcher();
        let term = tantivy::Term::from_field_text(self.id_field, id);
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);
        
        let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1))?;
        
        if let Some((_, doc_address)) = top_docs.first() {
            let doc: TantivyDocument = searcher.doc(*doc_address)?;
            
            return Ok(Some(IndexedDocument {
                id: get_text(&doc, self.id_field)?,
                doc_type: serde_json::from_str(&get_text(&doc, self.doc_type_field)?)
                    .unwrap_or(crate::schema::DocType::Message),
                title: get_optional_text(&doc, self.title_field),
                content: get_text(&doc, self.content_field)?,
                tags: get_all_text(&doc, self.tags_field),
                author: get_optional_text(&doc, self.author_field),
                source: get_optional_text(&doc, self.source_field),
                url: get_optional_text(&doc, self.url_field),
                created_at: get_i64(&doc, self.created_at_field)?,
                updated_at: get_i64(&doc, self.updated_at_field)?,
                session_id: get_optional_text(&doc, self.session_id_field),
                access_count: get_i64(&doc, self.access_count_field)?,
            }));
        }
        
        Ok(None)
    }

    pub async fn increment_access(&self, id: &str) -> Result<()> {
        if let Some(mut doc) = self.get(id).await? {
            doc.access_count += 1;
            self.update_document(doc).await?;
        }
        Ok(())
    }
}

// Helper functions
fn get_text(doc: &TantivyDocument, field: Field) -> Result<String> {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .context("Field not found or not a string")
}

fn get_optional_text(doc: &TantivyDocument, field: Field) -> Option<String> {
    doc.get_first(field).and_then(|v| v.as_str().map(|s| s.to_string()))
}

fn get_i64(doc: &TantivyDocument, field: Field) -> Result<i64> {
    doc.get_first(field)
        .and_then(|v| v.as_i64())
        .context("Field not found or not an i64")
}

fn get_all_text(doc: &TantivyDocument, field: Field) -> Vec<String> {
    doc.get_all(field)
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

fn generate_highlights(content: &str, query: &str, max_fragments: usize) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let content_lower = content.to_lowercase();
    
    let mut highlights = Vec::new();
    let words: Vec<&str> = query_lower.split_whitespace().collect();
    
    for word in words {
        if let Some(pos) = content_lower.find(word) {
            let start = pos.saturating_sub(50);
            let end = (pos + word.len() + 50).min(content.len());
            let fragment = &content[start..end];
            
            if start > 0 {
                highlights.push(format!("...{}", fragment));
            } else {
                highlights.push(format!("{}", fragment));
            }
            
            if highlights.len() >= max_fragments {
                break;
            }
        }
    }
    
    if highlights.is_empty() && !content.is_empty() {
        highlights.push(content.chars().take(150).collect::<String>());
    }
    
    highlights
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_add_and_search() {
        let dir = tempdir().unwrap();
        let index = SearchIndex::new(dir.path().join("index")).unwrap();
        
        let doc = IndexedDocument::new(
            "doc1".to_string(),
            DocType::Message,
            "Hello world, this is a test message".to_string(),
        )
        .with_title("Test Document".to_string())
        .with_tags(vec!["test".to_string(), "hello".to_string()]);
        
        index.add_document(doc).await.unwrap();
        index.commit().await.unwrap();
        
        let results = index
            .search(SearchQuery::new("hello world"), SearchOptions::default())
            .await
            .unwrap();
        
        assert_eq!(results.total, 1);
        assert_eq!(results.results[0].id, "doc1");
    }
    
    #[tokio::test]
    async fn test_fuzzy_search() {
        let dir = tempdir().unwrap();
        let index = SearchIndex::new(dir.path().join("index")).unwrap();
        
        let doc = IndexedDocument::new(
            "doc1".to_string(),
            DocType::Message,
            "The quick brown fox jumps".to_string(),
        );
        
        index.add_document(doc).await.unwrap();
        index.commit().await.unwrap();
        
        let results = index.search_fuzzy("quikc", 2, 10).await.unwrap();
        
        assert!(!results.is_empty());
    }
    
    #[tokio::test]
    async fn test_autocomplete() {
        let dir = tempdir().unwrap();
        let index = SearchIndex::new(dir.path().join("index")).unwrap();
        
        let doc = IndexedDocument::new(
            "doc1".to_string(),
            DocType::Message,
            "Python programming language".to_string(),
        );
        
        index.add_document(doc).await.unwrap();
        index.commit().await.unwrap();
        
        let results = index.autocomplete("pyt", 10).await.unwrap();
        
        assert!(!results.suggestions.is_empty());
    }
}
