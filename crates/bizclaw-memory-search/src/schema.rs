//! Tantivy schema definitions

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::doc;

pub fn create_schema() -> Schema {
    let mut schema_builder = Schema::builder();
    
    // Required fields
    schema_builder.add_text_field("id", STRING | STORED);
    schema_builder.add_text_field("doc_type", STRING | STORED);
    
    // Content fields for search
    let text_field_indexing = TextFieldIndexing::default()
        .set_tokenizer("default")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    
    let text_options = TextOptions::default()
        .set_indexing_options(text_field_indexing)
        .set_stored();
    
    schema_builder.add_text_field("title", text_options.clone());
    schema_builder.add_text_field("content", text_options.clone());
    schema_builder.add_text_field("tags", text_options.clone());
    schema_builder.add_text_field("author", text_options.clone());
    
    // Metadata fields (stored, not indexed for search)
    schema_builder.add_text_field("source", STRING | STORED);
    schema_builder.add_text_field("url", STRING | STORED);
    schema_builder.add_i64_field("created_at", INDEXED | STORED | FAST);
    schema_builder.add_i64_field("updated_at", INDEXED | STORED | FAST);
    schema_builder.add_text_field("session_id", STRING | STORED);
    schema_builder.add_i64_field("access_count", INDEXED | STORED | FAST);
    schema_builder.add_f64_field("relevance_score", STORED);
    
    schema_builder.build()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DocType {
    Message,
    Document,
    Memory,
    Knowledge,
    Config,
    Cache,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::Message => "message",
            DocType::Document => "document",
            DocType::Memory => "memory",
            DocType::Knowledge => "knowledge",
            DocType::Config => "config",
            DocType::Cache => "cache",
        }
    }
}

impl std::fmt::Display for DocType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexedDocument {
    pub id: String,
    pub doc_type: DocType,
    pub title: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub session_id: Option<String>,
    pub access_count: i64,
}

impl IndexedDocument {
    pub fn new(id: String, doc_type: DocType, content: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            doc_type,
            title: None,
            content,
            tags: Vec::new(),
            author: None,
            source: None,
            url: None,
            created_at: now,
            updated_at: now,
            session_id: None,
            access_count: 0,
        }
    }
    
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
    
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}
