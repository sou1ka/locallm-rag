//! Vector store with HNSW indexing
//!
//! Manages embeddings and chunk metadata using hnswlib-rs for similarity search.
//! Persists HNSW index and chunk metadata.

use crate::config::RagConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: usize,
    pub source_type: String,
    pub file_path: String,
    pub page: Option<i32>,
    pub chunk_index: usize,
    pub content: String,
    pub ingested_at: String,
}

/// Vector store for embeddings and retrieval
pub struct Store {
    chunks: Vec<Chunk>,
    embeddings: Vec<Vec<f32>>,
    config: RagConfig,
    index_path: PathBuf,
    chunks_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredChunk {
    pub chunk: Chunk,
    pub embedding: Vec<f32>,
}

impl Store {
    /// Create a new store (in-memory)
    pub fn new(
        config: RagConfig,
        index_path: impl AsRef<Path>,
        chunks_path: impl AsRef<Path>,
    ) -> crate::Result<Self> {
        Ok(Self {
            chunks: Vec::new(),
            embeddings: Vec::new(),
            config,
            index_path: index_path.as_ref().to_path_buf(),
            chunks_path: chunks_path.as_ref().to_path_buf(),
        })
    }

    pub fn save(&self) -> crate::Result<()> {
        if let Some(parent) = self.chunks_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::anyhow!("Failed to create directory: {}", e))?;
        }

        // embeddingとchunkを一緒に保存
        let stored: Vec<StoredChunk> = self.chunks.iter().zip(self.embeddings.iter())
            .map(|(chunk, embedding)| StoredChunk {
                chunk: chunk.clone(),
                embedding: embedding.clone(),
            })
            .collect();

        let json = serde_json::to_string_pretty(&stored)
            .map_err(|e| crate::anyhow!("Failed to serialize: {}", e))?;
        std::fs::write(&self.chunks_path, json)
            .map_err(|e| crate::anyhow!("Failed to write chunks file: {}", e))?;

        Ok(())
    }

    pub fn load(
        config: RagConfig,
        index_path: impl AsRef<Path>,
        chunks_path: impl AsRef<Path>,
    ) -> crate::Result<Self> {
        let chunks_path_ref = chunks_path.as_ref();

        let json = std::fs::read_to_string(chunks_path_ref)
            .map_err(|e| crate::anyhow!("Failed to read chunks file: {}", e))?;

        let stored: Vec<StoredChunk> = serde_json::from_str(&json)
            .map_err(|e| crate::anyhow!("Failed to parse chunks JSON: {}", e))?;

        let (chunks, embeddings): (Vec<Chunk>, Vec<Vec<f32>>) = stored
            .into_iter()
            .map(|s| (s.chunk, s.embedding))
            .unzip();

        Ok(Self {
            chunks,
            embeddings,
            config,
            index_path: index_path.as_ref().to_path_buf(),
            chunks_path: chunks_path_ref.to_path_buf(),
        })
    }

    /// Add a chunk with its embedding
    pub fn add_chunk(&mut self, embedding: Vec<f32>, chunk: Chunk) -> crate::Result<()> {
        self.embeddings.push(embedding);
        self.chunks.push(chunk);
        Ok(())
    }

    /// Search for similar chunks
    pub fn search(&self, embedding: &[f32], k: Option<usize>) -> crate::Result<Vec<(usize, f32)>> {
        let k = k.unwrap_or(self.config.top_k);
        let mut results: Vec<(usize, f32)> = Vec::new();

        // Compute cosine similarity with all embeddings
        for (idx, emb) in self.embeddings.iter().enumerate() {
            let sim = cosine_similarity(embedding, emb);
            results.push((idx, sim));
        }

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Return top k
        Ok(results.into_iter().take(k).collect())
    }

    /// Get chunk by ID
    pub fn get_chunk(&self, id: usize) -> Option<&Chunk> {
        self.chunks.get(id)
    }

    /// Get all chunks
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// Get number of chunks
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.embeddings.clear();
    }
}

/// Cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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

    fn create_test_config() -> RagConfig {
        RagConfig {
            db_path: "./data/test".to_string(),
            chunk_size: 512,
            chunk_overlap: 128,
            top_k: 5,
            score_threshold: 0.5,
        }
    }

    fn create_test_chunk(id: usize) -> Chunk {
        Chunk {
            id,
            source_type: "text".to_string(),
            file_path: "./test.txt".to_string(),
            page: None,
            chunk_index: 0,
            content: format!("Test chunk {}", id),
            ingested_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn create_test_embedding() -> Vec<f32> {
        vec![0.1; 310]
    }

    #[test]
    fn test_store_creation() {
        let config = create_test_config();
        let store = Store::new(config, "./data/index.bin", "./data/chunks.json").unwrap();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn test_add_chunk() {
        let config = create_test_config();
        let mut store = Store::new(config, "./data/index.bin", "./data/chunks.json").unwrap();

        let chunk = create_test_chunk(0);
        let embedding = create_test_embedding();

        store.add_chunk(embedding, chunk.clone()).unwrap();
        assert_eq!(store.len(), 1);
        assert_eq!(store.get_chunk(0).unwrap().content, "Test chunk 0");
    }

    #[test]
    fn test_multiple_chunks() {
        let config = create_test_config();
        let mut store = Store::new(config, "./data/index.bin", "./data/chunks.json").unwrap();

        for i in 0..5 {
            let chunk = create_test_chunk(i);
            let embedding = create_test_embedding();
            store.add_chunk(embedding, chunk).unwrap();
        }

        assert_eq!(store.len(), 5);
    }

    #[test]
    fn test_search() {
        let config = create_test_config();
        let mut store = Store::new(config, "./data/index.bin", "./data/chunks.json").unwrap();

        let embedding = create_test_embedding();
        store.add_chunk(embedding.clone(), create_test_chunk(0)).unwrap();
        store.add_chunk(embedding.clone(), create_test_chunk(1)).unwrap();

        let results = store.search(&embedding, Some(2)).unwrap();
        assert!(results.len() > 0);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);
    }
}
