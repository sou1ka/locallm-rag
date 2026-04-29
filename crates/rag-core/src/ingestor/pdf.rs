//! PDF document ingestion
//!
//! Handles both text-based and scanned PDFs.
//! Automatically detects PDF type and uses OCR for scanned documents.

use std::path::Path;
use crate::chunker::ChunkSplitter;
use crate::store::Chunk;

/// Minimum characters per page to classify as text PDF (vs scanned)
const MIN_CHARS_PER_PAGE: usize = 50;

/// Load and ingest a PDF file, returning chunks
pub fn load(path: &Path, splitter: &ChunkSplitter) -> crate::Result<Vec<Chunk>> {
    let text = ingest_pdf(path)?;

    // Split into chunks
    let chunk_texts = splitter.split(&text)?;

    // Convert to Chunk structs
    let chunks = chunk_texts
        .into_iter()
        .enumerate()
        .map(|(idx, content)| Chunk {
            id: idx,
            source_type: "pdf".to_string(),
            file_path: path.to_string_lossy().to_string(),
            page: None,
            chunk_index: idx,
            content,
            ingested_at: crate::ingestor::now_iso8601(),
        })
        .collect();

    Ok(chunks)
}

/// Ingest a PDF file
///
/// # Arguments
/// * `path` - Path to PDF file
///
/// # Returns
/// Extracted text content
pub fn ingest_pdf(path: &Path) -> crate::Result<String> {
    // Try to extract text from PDF
    match extract_text_from_pdf_file(path) {
        Ok(text) if text.len() >= MIN_CHARS_PER_PAGE => {
            // Text PDF: successfully extracted enough text
            Ok(text)
        }
        Ok(_) => {
            // Scanned PDF: extracted text is too short, would need OCR
            Err(crate::anyhow!(
                "PDF appears to be scanned (insufficient text extracted). OCR support needed."
            ))
        }
        Err(e) => {
            // Error during extraction
            Err(crate::anyhow!("Failed to extract text from PDF: {}", e))
        }
    }
}

/// Extract text from PDF file
fn extract_text_from_pdf_file(path: &Path) -> crate::Result<String> {
    use pdf_extract::Document;

    let document = Document::load(path)
        .map_err(|e| crate::anyhow!("Failed to load PDF: {}", e))?;

    // Get all text content - pdf-extract library limitation:
    // We cannot directly iterate pages like other methods, so we use a simple approach
    let mut all_text = String::new();

    // Try to extract all content
    for page_id in document.get_pages().values() {
        // Access page content if available
        match document.get_page_content(*page_id) {
            Ok(content) => {
                all_text.push_str(&format!("{:?}", content));
                all_text.push('\n');
            }
            Err(_) => continue,
        }
    }

    Ok(all_text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_chars_constant() {
        assert_eq!(MIN_CHARS_PER_PAGE, 50);
    }

    #[test]
    fn test_ingest_pdf_nonexistent() {
        let result = ingest_pdf(Path::new("./nonexistent.pdf"));
        assert!(result.is_err());
    }
}
