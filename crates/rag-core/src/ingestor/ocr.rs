//! OCR (Optical Character Recognition) module
//!
//! Wrapper around leptess (Tesseract) for extracting text from images.
//! Supports Japanese, English, and mixed-language documents.

use std::path::Path;

/// OCR configuration
#[derive(Debug, Clone)]
pub struct OcrConfig {
    pub tessdata_path: Option<String>,
    pub lang: String,
}

impl OcrConfig {
    /// Create OCR config with Japanese language
    pub fn japanese(tessdata_path: Option<String>) -> Self {
        Self {
            tessdata_path,
            lang: "jpn".to_string(),
        }
    }

    /// Create OCR config with Japanese and English
    pub fn japanese_english(tessdata_path: Option<String>) -> Self {
        Self {
            tessdata_path,
            lang: "jpn+eng".to_string(),
        }
    }

    /// Create OCR config with English only
    pub fn english(tessdata_path: Option<String>) -> Self {
        Self {
            tessdata_path,
            lang: "eng".to_string(),
        }
    }
}

/// Extract text from an image using OCR
///
/// # Arguments
/// * `image_path` - Path to image file (jpg, png, tiff, etc)
/// * `config` - OCR configuration
///
/// # Returns
/// Extracted text from the image
pub fn ocr_image(image_path: &Path, config: &OcrConfig) -> crate::Result<String> {
    // Check file exists
    if !image_path.exists() {
        return Err(crate::anyhow!(
            "Image file not found: {}",
            image_path.display()
        ));
    }

    // Initialize Tesseract with leptess
    let tessdata_ref = config.tessdata_path.as_deref();
    let mut lt = leptess::LepTess::new(tessdata_ref, &config.lang)
        .map_err(|e| crate::anyhow!("Failed to initialize Tesseract: {}", e))?;

    // Set image
    lt.set_image(image_path.to_str().unwrap());

    // Extract text
    let text = lt
        .get_utf8_text()
        .map_err(|e| crate::anyhow!("OCR failed: {}", e))?;

    Ok(text.trim().to_string())
}

/// Extract text from image bytes using OCR
///
/// # Arguments
/// * `image_bytes` - Image data in memory
/// * `config` - OCR configuration
///
/// # Returns
/// Extracted text from the image
pub fn ocr_image_bytes(image_bytes: &[u8], config: &OcrConfig) -> crate::Result<String> {
    // For now, write to temporary file and use ocr_image
    // In the future, leptess might support in-memory images directly
    
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("ocr_temp.png");

    // Write bytes to temporary file
    std::fs::write(&temp_path, image_bytes)
        .map_err(|e| crate::anyhow!("Failed to write temporary image: {}", e))?;

    // Process with OCR
    let result = ocr_image(&temp_path, config)?;

    // Clean up
    let _ = std::fs::remove_file(&temp_path);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_config_japanese() {
        let config = OcrConfig::japanese(None);
        assert_eq!(config.lang, "jpn");
        assert!(config.tessdata_path.is_none());
    }

    #[test]
    fn test_ocr_config_japanese_english() {
        let config = OcrConfig::japanese_english(Some("./tessdata".to_string()));
        assert_eq!(config.lang, "jpn+eng");
        assert_eq!(config.tessdata_path, Some("./tessdata".to_string()));
    }

    #[test]
    fn test_ocr_config_english() {
        let config = OcrConfig::english(None);
        assert_eq!(config.lang, "eng");
    }

    #[test]
    fn test_ocr_image_nonexistent() {
        let config = OcrConfig::japanese(None);
        let result = ocr_image(Path::new("./nonexistent.png"), &config);
        assert!(result.is_err());
    }
}
