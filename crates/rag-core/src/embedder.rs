//! Embedding Module
//!
//! Wraps fastembed-rs for document and query vectorization.
//! Uses ruri-v3-310m (Japanese-optimized) ONNX model.

use crate::config::EmbedderConfig;
use fastembed::TextEmbedding;
use std::path::Path;

/// Embedder wrapper for fastembed with custom ONNX models
pub struct Embedder {
    model: TextEmbedding,
    config: EmbedderConfig,
}

impl Embedder {
    /// Create embedder from config
    ///
    /// Loads ruri-v3-310m ONNX model from file paths specified in config.
    pub fn new(config: EmbedderConfig) -> crate::Result<Self> {
        let onnx_path = Path::new(&config.onnx_path);
        let tokenizer_path = Path::new(&config.tokenizer_path);
        // モデルディレクトリ（onnx_pathの親ディレクトリを想定）
        let model_dir = onnx_path
            .parent()
            .ok_or_else(|| crate::anyhow!("Invalid onnx_path"))?;

        let read = |filename: &str| -> crate::Result<Vec<u8>> {
            let p = model_dir.join(filename);
            std::fs::read(&p)
                .map_err(|e| crate::anyhow!("Failed to read {}: {}", p.display(), e))
        };

        let onnx_bytes      = std::fs::read(onnx_path)
            .map_err(|e| crate::anyhow!("Failed to read ONNX: {}", e))?;
        let tokenizer_bytes = std::fs::read(tokenizer_path)
            .map_err(|e| crate::anyhow!("Failed to read tokenizer: {}", e))?;
        let config_bytes             = read("config.json")?;
        let special_tokens_bytes     = read("special_tokens_map.json")?;
        let tokenizer_config_bytes   = read("tokenizer_config.json")?;

        let token_files = fastembed::TokenizerFiles {
            tokenizer_file:         tokenizer_bytes,
            config_file:            config_bytes,
            special_tokens_map_file: special_tokens_bytes,
            tokenizer_config_file:  tokenizer_config_bytes,
        };

        // non_exhaustiveのためstructリテラル不可 → マクロ相当の手動組み立て
        // UserDefinedEmbeddingModelはDerefできないのでDefaultを経由して上書き
        let user_model = fastembed::UserDefinedEmbeddingModel::new(onnx_bytes, token_files);

        let model = TextEmbedding::try_new_from_user_defined(
            user_model,
            fastembed::InitOptionsUserDefined::default(),
        )
        .map_err(|e| crate::anyhow!("Failed to load embedding model: {}", e))?;

        Ok(Self { model, config })
    }

    /// Embed document with doc_prefix
    pub fn embed_document(&self, text: &str) -> crate::Result<Vec<f32>> {
        //let prefixed = format!("{}{}", self.config.doc_prefix, text);
        //self.embed_text(&prefixed)
        let prefixed = format!("{}{}", self.config.doc_prefix, text);
        let result = self.embed_text(&prefixed)?;
        // デバッグ出力
        //let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        //eprintln!("[DEBUG] embed_document norm={:.4} dim={}", norm, result.len());
        Ok(result)
    }

    /// Embed query with query_prefix
    pub fn embed_query(&self, text: &str) -> crate::Result<Vec<f32>> {
        //let prefixed = format!("{}{}", self.config.query_prefix, text);
        //self.embed_text(&prefixed)
        let prefixed = format!("{}{}", self.config.query_prefix, text);
        let result = self.embed_text(&prefixed)?;
        // デバッグ出力
        //let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        //eprintln!("[DEBUG] embed_query norm={:.4} dim={}", norm, result.len());
        Ok(result)
    }

    /// Embed batch of documents
    pub fn embed_documents(&self, texts: Vec<&str>) -> crate::Result<Vec<Vec<f32>>> {
        let prefixed: Vec<String> = texts
            .iter()
            .map(|t| format!("{}{}", self.config.doc_prefix, t))
            .collect();
        let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
        self.embed_batch(&refs)
    }

    /// Embed batch of queries
    pub fn embed_queries(&self, texts: Vec<&str>) -> crate::Result<Vec<Vec<f32>>> {
        let prefixed: Vec<String> = texts
            .iter()
            .map(|t| format!("{}{}", self.config.query_prefix, t))
            .collect();
        let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
        self.embed_batch(&refs)
    }

    fn embed_text(&self, text: &str) -> crate::Result<Vec<f32>> {
        self.model
            .embed(vec![text], None)
            .map_err(|e| crate::anyhow!("Embedding failed: {}", e))?
            .into_iter()
            .next()
            .ok_or_else(|| crate::anyhow!("No embeddings returned"))
    }

    fn embed_batch(&self, texts: &[&str]) -> crate::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        self.model
            .embed(texts.to_vec(), Some(self.config.embed_batch_size))
            .map_err(|e| crate::anyhow!("Batch embedding failed: {}", e))
    }

    pub fn model_type(&self) -> &str {
        &self.config.model_type
    }

    pub fn doc_prefix(&self) -> &str {
        &self.config.doc_prefix
    }

    pub fn query_prefix(&self) -> &str {
        &self.config.query_prefix
    }
}

/// Cosine similarity between embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_mismatch() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_prefix_format() {
        let doc_prefix = "文章: ";
        let query_prefix = "クエリ: ";

        let doc = format!("{}{}", doc_prefix, "Hello");
        let query = format!("{}{}", query_prefix, "Hi");

        assert_eq!(doc, "文章: Hello");
        assert_eq!(query, "クエリ: Hi");
    }

    #[test]
    fn test_normalize() {
        let v = vec![3.0, 4.0];
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 5.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_scaled() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 4.0, 6.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.0001);
    }
}
