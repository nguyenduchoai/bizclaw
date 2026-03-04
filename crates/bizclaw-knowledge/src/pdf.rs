//! PDF extraction module — powered by pdf_oxide.
//!
//! Extracts text and markdown from PDF documents for RAG indexing.
//! Uses pdf_oxide's pipeline API for reading-order-aware extraction
//! that preserves document structure (headings, tables, lists).

/// Extract plain text from PDF bytes.
///
/// Iterates through all pages and concatenates extracted text.
/// This is the simplest extraction mode — no layout preservation.
pub fn extract_text_from_pdf(data: &[u8]) -> Result<String, String> {
    use pdf_oxide::PdfDocument;

    let mut doc = PdfDocument::open_from_bytes(data.to_vec())
        .map_err(|e| format!("PDF parse error: {e}"))?;

    let page_count = doc.page_count()
        .map_err(|e| format!("PDF page count error: {e}"))?;
    tracing::debug!("📄 PDF opened: {} pages", page_count);

    let mut text = String::new();
    for page_idx in 0..page_count {
        match doc.extract_text(page_idx) {
            Ok(page_text) => {
                if !page_text.is_empty() {
                    text.push_str(&page_text);
                    text.push('\n');
                }
            }
            Err(e) => {
                tracing::warn!("⚠️ PDF page {} text extraction failed: {e}", page_idx);
            }
        }
    }

    if text.trim().is_empty() {
        return Err("PDF contains no extractable text (may be scanned/image-only)".into());
    }

    tracing::info!("📄 PDF extracted: {} pages → {} chars", page_count, text.len());
    Ok(text)
}

/// Extract markdown from PDF bytes (higher quality for RAG).
///
/// Uses pdf_oxide's TextPipeline with reading-order detection and
/// MarkdownOutputConverter to preserve document structure.
/// Falls back to plain text extraction if markdown conversion fails.
pub fn extract_markdown_from_pdf(data: &[u8]) -> Result<String, String> {
    use pdf_oxide::PdfDocument;
    use pdf_oxide::pipeline::{TextPipeline, TextPipelineConfig};
    use pdf_oxide::pipeline::converters::{MarkdownOutputConverter, OutputConverter};

    let mut doc = PdfDocument::open_from_bytes(data.to_vec())
        .map_err(|e| format!("PDF parse error: {e}"))?;

    let page_count = doc.page_count()
        .map_err(|e| format!("PDF page count error: {e}"))?;
    tracing::debug!("📄 PDF opened for markdown extraction: {} pages", page_count);

    let config = TextPipelineConfig::default();
    let pipeline = TextPipeline::with_config(config.clone());
    let converter = MarkdownOutputConverter::new();

    let mut markdown = String::new();
    let mut fallback_count: usize = 0;

    for page_idx in 0..page_count {
        // Try markdown extraction via pipeline
        let page_result: Result<String, String> = (|| -> Result<String, String> {
            let spans = doc.extract_spans(page_idx)
                .map_err(|e| format!("spans: {e}"))?;
            let ordered = pipeline.process(spans, Default::default())
                .map_err(|e| format!("pipeline: {e}"))?;
            let md: String = converter.convert(&ordered, &config)
                .map_err(|e| format!("convert: {e}"))?;
            Ok(md)
        })();

        match page_result {
            Ok(page_md) => {
                if !page_md.trim().is_empty() {
                    markdown.push_str(&page_md);
                    markdown.push_str("\n\n");
                }
            }
            Err(_) => {
                // Fallback to plain text for this page
                fallback_count += 1;
                if let Ok(text) = doc.extract_text(page_idx) {
                    if !text.is_empty() {
                        markdown.push_str(&text);
                        markdown.push('\n');
                    }
                }
            }
        }
    }

    if markdown.trim().is_empty() {
        return Err("PDF contains no extractable text (may be scanned/image-only)".into());
    }

    if fallback_count > 0 {
        tracing::info!(
            "📄 PDF markdown: {} pages ({} plain text fallback) → {} chars",
            page_count, fallback_count, markdown.len()
        );
    } else {
        tracing::info!(
            "📄 PDF markdown: {} pages → {} chars",
            page_count, markdown.len()
        );
    }

    Ok(markdown)
}

/// Get PDF metadata (page count).
pub fn pdf_info(data: &[u8]) -> Result<PdfInfo, String> {
    use pdf_oxide::PdfDocument;

    let mut doc = PdfDocument::open_from_bytes(data.to_vec())
        .map_err(|e| format!("PDF parse error: {e}"))?;
    let page_count = doc.page_count()
        .map_err(|e| format!("PDF page count error: {e}"))?;

    Ok(PdfInfo { page_count })
}

/// Basic PDF metadata.
#[derive(Debug, Clone)]
pub struct PdfInfo {
    pub page_count: usize,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pdf_module_compiles() {
        assert!(true);
    }
}
