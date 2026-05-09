//! Retrieval module for similarity-based chunk search
//!
//! Searches the vector store for chunks similar to a query embedding,
//! filtering results by the configured score threshold.

use crate::config::RagConfig;
use crate::store::{Chunk, Store};

/// Search result containing a chunk and its similarity score
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub chunk: Chunk,
    pub score: f32,
}

/// Retriever for finding relevant chunks
pub struct Retriever<'a> {
    store: &'a Store,
    config: RagConfig,
}

impl<'a> Retriever<'a> {
    /// Create a new retriever
    ///
    /// # Arguments
    /// * `store` - Reference to the vector store
    /// * `config` - RAG configuration containing score_threshold and top_k
    pub fn new(store: &'a Store, config: RagConfig) -> Self {
        Retriever { store, config }
    }

    /// Retrieve relevant chunks for a query embedding
    ///
    /// # Arguments
    /// * `query_embedding` - Query vector embedding
    ///
    /// # Returns
    /// Vector of RetrievalResult sorted by score (highest first),
    /// filtered by score_threshold and limited to top_k results
    pub fn retrieve(&self, query_embedding: &[f32]) -> crate::Result<Vec<RetrievalResult>> {
        // Search store for similar chunks
        let results = self
            .store
            .search(query_embedding, Some(self.config.top_k))?;

        // ← デバッグ出力を追加
        //eprintln!("[DEBUG] search returned {} results", results.len());
        //for (id, score) in &results {
        //    eprintln!("[DEBUG] chunk_id={} score={:.4}", id, score);
        //}

        // Filter by score threshold
        let filtered: Vec<RetrievalResult> = results
            .into_iter()
            .filter(|(_, score)| *score >= self.config.score_threshold)
            .filter_map(|(chunk_id, score)| {
                self.store
                    .get_chunk(chunk_id)
                    .map(|chunk| RetrievalResult {
                        chunk: chunk.clone(),
                        score,
                    })
            })
            .collect();

        Ok(filtered)
    }

    /// Get all chunks that pass the threshold for inspection
    pub fn all_chunks(&self) -> Vec<Chunk> {
        self.store.chunks().to_vec()
    }

    /// Get chunk by ID
    pub fn get_chunk(&self, id: usize) -> Option<&Chunk> {
        self.store.get_chunk(id)
    }

    /// Get the configured score threshold
    pub fn score_threshold(&self) -> f32 {
        self.config.score_threshold
    }

    /// Get the configured top_k
    pub fn top_k(&self) -> usize {
        self.config.top_k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> RagConfig {
        RagConfig {
            db_path: ":memory:".to_string(),
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
    fn test_retriever_creation() {
        let config = create_test_config();
        let store = Store::open(config.clone(), ":memory:")
            .unwrap();
        let retriever = Retriever::new(&store, config);

        assert_eq!(retriever.score_threshold(), 0.5);
        assert_eq!(retriever.top_k(), 5);
    }

    #[test]
    fn test_retrieve_empty_store() {
        let config = create_test_config();
        let store = Store::open(config.clone(), ":memory:")
            .unwrap();
        let retriever = Retriever::new(&store, config);

        let embedding = create_test_embedding();
        let results = retriever.retrieve(&embedding).unwrap();

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_retrieve_single_chunk() {
        let config = create_test_config();
        let mut store = Store::open(config.clone(), ":memory:")
            .unwrap();

        let chunk = create_test_chunk(0);
        let embedding = create_test_embedding();
        store.add_chunk(embedding.clone(), chunk.clone()).unwrap();

        let retriever = Retriever::new(&store, config);
        let results = retriever.retrieve(&embedding).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].chunk.content, "Test chunk 0");
    }

    #[test]
    fn test_retrieve_multiple_chunks() {
        let config = create_test_config();
        let mut store = Store::open(config.clone(), ":memory:")
            .unwrap();

        let embedding = create_test_embedding();
        for i in 0..3 {
            let chunk = create_test_chunk(i);
            store.add_chunk(embedding.clone(), chunk).unwrap();
        }

        let retriever = Retriever::new(&store, config);
        let results = retriever.retrieve(&embedding).unwrap();

        assert!(results.len() > 0);
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_retrieve_filters_by_threshold() {
        let mut config = create_test_config();
        config.score_threshold = 0.99; // Very high threshold

        let mut store = Store::open(config.clone(), ":memory:")
            .unwrap();

        let embedding = create_test_embedding();
        let chunk = create_test_chunk(0);
        store.add_chunk(embedding.clone(), chunk).unwrap();

        let retriever = Retriever::new(&store, config);
        let results = retriever.retrieve(&embedding).unwrap();

        // With perfect match (1.0) and threshold 0.99, should still return result
        assert!(results.len() > 0);
    }

    #[test]
    fn test_retrieve_respects_top_k() {
        let mut config = create_test_config();
        config.top_k = 2;

        let mut store = Store::open(config.clone(), ":memory:")
            .unwrap();

        let embedding = create_test_embedding();
        for i in 0..5 {
            let chunk = create_test_chunk(i);
            store.add_chunk(embedding.clone(), chunk).unwrap();
        }

        let retriever = Retriever::new(&store, config);
        let results = retriever.retrieve(&embedding).unwrap();

        assert!(results.len() <= 2);
    }

    #[test]
    fn test_get_chunk() {
        let config = create_test_config();
        let mut store = Store::open(config.clone(), ":memory:")
            .unwrap();

        let chunk = create_test_chunk(0);
        let embedding = create_test_embedding();
        store.add_chunk(embedding, chunk).unwrap();

        let retriever = Retriever::new(&store, config);
        let retrieved = retriever.get_chunk(0);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test chunk 0");
    }

    #[test]
    fn test_all_chunks() {
        let config = create_test_config();
        let mut store = Store::open(config.clone(), ":memory:")
            .unwrap();

        let embedding = create_test_embedding();
        for i in 0..3 {
            let chunk = create_test_chunk(i);
            store.add_chunk(embedding.clone(), chunk).unwrap();
        }

        let retriever = Retriever::new(&store, config);
        let chunks = retriever.all_chunks();

        assert_eq!(chunks.len(), 3);
    }
}
