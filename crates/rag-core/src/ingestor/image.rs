//! Image file ingestion module
//!
//! Handles image files (.jpg, .jpeg, .png, .tiff, .tif, .webp)
//! Performs optical character recognition (OCR) to extract text.

use std::path::Path;
use crate::store::Chunk;

/// Load and ingest an image file, returning chunks
/// 
/// Image files are processed for OCR without splitting (single chunk per file)
pub fn load(path: &Path) -> crate::Result<Vec<Chunk>> {
    let text = ingest_image(path)?;

    // Image OCR typically produces a single chunk per file
    let chunk = Chunk {
        id: 0,
        source_type: "image".to_string(),
        file_path: path.to_string_lossy().to_string(),
        page: None,
        chunk_index: 0,
        content: text,
        ingested_at: crate::ingestor::now_iso8601(),
    };

    Ok(vec![chunk])
}

/// Ingest an image file
///
/// # Arguments
/// * `path` - Path to image file
///
/// # Returns
/// Extracted text from image via OCR
pub fn ingest_image(path: &Path) -> crate::Result<String> {
    // Check file exists
    if !path.exists() {
        return Err(crate::anyhow!(
            "Image file not found: {}",
            path.display()
        ));
    }

    // Check file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "tiff" | "tif" | "webp" => {
            // OCR processing would be implemented here
            // Could use tesseract or other OCR library
            // For now, placeholder implementation
            Err(crate::anyhow!(
                "OCR processing not yet implemented for file: {}",
                path.display()
            ))
        }
        _ => Err(crate::anyhow!("Unsupported image format: {}", ext)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_image_nonexistent() {
        let result = ingest_image(Path::new("./nonexistent.jpg"));
        assert!(result.is_err());
    }
}
