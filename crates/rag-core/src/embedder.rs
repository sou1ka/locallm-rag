//! Embedding Module
//!
//! Provides text embedding using fastembed-rs with custom ONNX models.
//! Supports ruri-v3-310m (Japanese-optimized) and other ONNX models.

use crate::config::EmbedderConfig;
use fastembed::{TextEmbedding, UserDefinedEmbeddingModel, TokenizerFiles};
use std::path::PathBuf;

/// Text embedding wrapper using fastembed
pub struct Embedder {
    model: TextEmbedding,
    config: EmbedderConfig,
}

impl Embedder {
    /// Create a new embedder from configuration
    ///
    /// Loads custom ONNX model specified in config (e.g., ruri-v3-310m)
    ///
    /// # Errors
    /// Returns error if model or tokenizer files don't exist or fail to load
    pub fn new(config: EmbedderConfig) -> crate::Result<Self> {
        // Convert paths to PathBuf
        let onnx_path = PathBuf::from(&config.onnx_path);
        let tokenizer_path = PathBuf::from(&config.tokenizer_path);

        // Verify model files exist
        if !onnx_path.exists() {
            return Err(crate::anyhow!(
                "ONNX model file not found: {}",
                onnx_path.display()
            ));
        }
        if !tokenizer_path.exists() {
            return Err(crate::anyhow!(
                "Tokenizer file not found: {}",
                tokenizer_path.display()
            ));
        }

        // Load custom model
        let model = TextEmbedding::try_new_from_user_defined(
            UserDefinedEmbeddingModel {
                onnx_file: onnx_path,
                tokenizer_files: TokenizerFiles {
                    tokenizer_file: tokenizer_path,
                    ..Default::default()
                },
            },
            Default::default(),
        ).map_err(|e| {
            crate::anyhow!("Failed to load embedding model: {}", e)
        })?;

        Ok(Self { model, config })
    }

    /// Embed a document with the document prefix
    ///
    /// Automatically adds doc_prefix to the text for optimal embedding quality
    /// (required for ruri-v3 and similar Japanese models)
    ///
    /// # Arguments
    /// * `text` - Document text to embed
    ///
    /// # Returns
    /// Embedding vector (typically 768 or 1024 dimensions)
    pub fn embed_document(&self, text: &str) -> crate::Result<Vec<f32>> {
        let prefixed_text = format!("{}{}", self.config.doc_prefix, text);
        self.embed_text(&prefixed_text)
    }

    /// Embed a query with the query prefix
    ///
    /// Automatically adds query_prefix to the text for optimal embedding quality
    /// (required for ruri-v3 and similar Japanese models)
    ///
    /// # Arguments
    /// * `text` - Query text to embed
    ///
    /// # Returns
    /// Embedding vector (typically 768 or 1024 dimensions)
    pub fn embed_query(&self, text: &str) -> crate::Result<Vec<f32>> {
        let prefixed_text = format!("{}{}", self.config.query_prefix, text);
        self.embed_text(&prefixed_text)
    }

    /// Embed multiple documents efficiently
    ///
    /// Batch processing is more efficient than calling embed_document repeatedly
    ///
    /// # Arguments
    /// * `texts` - Document texts to embed
    ///
    /// # Returns
    /// Vector of embedding vectors
    pub fn embed_documents(&self, texts: Vec<&str>) -> crate::Result<Vec<Vec<f32>>> {
        let prefixed_texts: Vec<String> = texts
            .iter()
            .map(|text| format!("{}{}", self.config.doc_prefix, text))
            .collect();

        let text_refs: Vec<&str> = prefixed_texts.iter().map(|s| s.as_str()).collect();
        self.embed_batch(&text_refs)
    }

    /// Embed multiple queries efficiently
    ///
    /// Batch processing is more efficient than calling embed_query repeatedly
    ///
    /// # Arguments
    /// * `texts` - Query texts to embed
    ///
    /// # Returns
    /// Vector of embedding vectors
    pub fn embed_queries(&self, texts: Vec<&str>) -> crate::Result<Vec<Vec<f32>>> {
        let prefixed_texts: Vec<String> = texts
            .iter()
            .map(|text| format!("{}{}", self.config.query_prefix, text))
            .collect();

        let text_refs: Vec<&str> = prefixed_texts.iter().map(|s| s.as_str()).collect();
        self.embed_batch(&text_refs)
    }

    /// Internal method to embed a single text
    fn embed_text(&self, text: &str) -> crate::Result<Vec<f32>> {
        let embeddings = self.model.embed(vec![text], None)
            .map_err(|e| crate::anyhow!("Embedding failed: {}", e))?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| crate::anyhow!("No embeddings returned from model"))
    }

    /// Internal method to embed multiple texts in batch
    fn embed_batch(&self, texts: &[&str]) -> crate::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let embeddings = self.model.embed(texts.to_vec(), None)
            .map_err(|e| crate::anyhow!("Batch embedding failed: {}", e))?;

        Ok(embeddings)
    }

    /// Get embedding model information
    pub fn model_info(&self) -> EmbedderInfo {
        EmbedderInfo {
            model_type: self.config.model_type.clone(),
            onnx_path: self.config.onnx_path.clone(),
            tokenizer_path: self.config.tokenizer_path.clone(),
            doc_prefix: self.config.doc_prefix.clone(),
            query_prefix: self.config.query_prefix.clone(),
        }
    }
}

/// Information about the loaded embedding model
#[derive(Debug, Clone)]
pub struct EmbedderInfo {
    pub model_type: String,
    pub onnx_path: String,
    pub tokenizer_path: String,
    pub doc_prefix: String,
    pub query_prefix: String,
}

/// Compute cosine similarity between two embedding vectors
///
/// Returns value between -1.0 and 1.0 (typically -1.0 to 1.0, often 0.0 to 1.0 for embeddings)
///
/// # Arguments
/// * `a` - First embedding vector
/// * `b` - Second embedding vector
///
/// # Returns
/// Cosine similarity score
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.0001, "Identical vectors should have similarity 1.0");
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.0001, "Orthogonal vectors should have similarity 0.0");
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 0.0001, "Opposite vectors should have similarity -1.0");
    }

    #[test]
    fn test_cosine_similarity_45degree() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 1.0].iter().map(|x| x / 2.0_f32.sqrt()).collect::<Vec<_>>();
        let sim = cosine_similarity(&a, &b);
        let expected_cos_45 = 45.0_f32.to_radians().cos();
        assert!((sim - expected_cos_45).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_length_mismatch() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_embedder_info_creation() {
        let config = EmbedderConfig {
            model_type: "ruri-v3".to_string(),
            onnx_path: "./models/ruri-v3.onnx".to_string(),
            tokenizer_path: "./models/tokenizer.json".to_string(),
            doc_prefix: "文章: ".to_string(),
            query_prefix: "クエリ: ".to_string(),
        };

        let info = EmbedderInfo {
            model_type: config.model_type.clone(),
            onnx_path: config.onnx_path.clone(),
            tokenizer_path: config.tokenizer_path.clone(),
            doc_prefix: config.doc_prefix.clone(),
            query_prefix: config.query_prefix.clone(),
        };

        assert_eq!(info.model_type, "ruri-v3");
        assert_eq!(info.doc_prefix, "文章: ");
        assert_eq!(info.query_prefix, "クエリ: ");
    }

    #[test]
    fn test_prefix_application() {
        let doc_prefix = "文章: ";
        let query_prefix = "クエリ: ";

        let doc_text = "This is a document";
        let query_text = "What is this?";

        let prefixed_doc = format!("{}{}", doc_prefix, doc_text);
        let prefixed_query = format!("{}{}", query_prefix, query_text);

        assert_eq!(prefixed_doc, "文章: This is a document");
        assert_eq!(prefixed_query, "クエリ: What is this?");
    }

    #[test]
    fn test_normalize_vector() {
        let v = vec![3.0, 4.0];
        let norm = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
        assert!((norm - 5.0).abs() < 0.0001);
    }
}