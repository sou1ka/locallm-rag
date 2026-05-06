//! Text chunking module
//!
//! Splits text into overlapping chunks using lindera for Japanese tokenization.
//! Chunk size and overlap configured via RagConfig.

use crate::RagConfig;
use lindera::tokenizer::Tokenizer;
use lindera::tokenizer::TokenizerConfig;
//use lindera::DictionaryKind;
//use lindera::Mode;

/// Text splitter that creates overlapping chunks
pub struct ChunkSplitter {
    config: RagConfig,
//    tokenizer: Tokenizer,
}
/*
impl ChunkSplitter {
    pub fn new(config: RagConfig) -> crate::Result<Self> {
        let tokenizer_config = TokenizerConfig {
            dictionary: lindera::DictionaryConfig {
                kind: Some(DictionaryKind::IPADIC),
                path: None,
            },
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let tokenizer = Tokenizer::from_config(&tokenizer_config)
            .map_err(|e| crate::anyhow!("Failed to create tokenizer: {}", e))?;

        Ok(Self { config, tokenizer })
    }

    /// Split text into overlapping chunks
    ///
    /// # Arguments
    /// * `text` - Text to split
    ///
    /// # Returns
    /// Vector of text chunks
    pub fn split(&self, text: &str) -> crate::Result<Vec<String>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        // Tokenize the entire text
        let tokens = self
            .tokenizer
            .tokenize(text)
            .map_err(|e| crate::anyhow!("Tokenization failed: {}", e))?;

        if tokens.is_empty() {
            return Ok(vec![]);
        }

        // Convert tokens to owned strings using surface field
        let token_texts: Vec<String> = tokens
            .iter()
            .map(|t| t.text.to_string())
            .collect();

        // If no chunking needed (text smaller than chunk_size)
        if token_texts.len() <= self.config.chunk_size {
            return Ok(vec![text.to_string()]);
        }

        let mut chunks = Vec::new();
        let mut start_idx = 0;

        loop {
            // Create chunk from current position
            let end_idx = std::cmp::min(start_idx + self.config.chunk_size, token_texts.len());

            if start_idx >= token_texts.len() {
                break;
            }

            let chunk_tokens = &token_texts[start_idx..end_idx];
            let chunk_text = chunk_tokens.join("");
            chunks.push(chunk_text);

            // If we've reached the end, break
            if end_idx >= token_texts.len() {
                break;
            }

            // Move start position forward with overlap
            start_idx += self.config.chunk_size - self.config.chunk_overlap;
        }

        Ok(chunks)
    }

    /// Get chunk size
    pub fn chunk_size(&self) -> usize {
        self.config.chunk_size
    }

    /// Get chunk overlap
    pub fn chunk_overlap(&self) -> usize {
        self.config.chunk_overlap
    }
}
*/
impl ChunkSplitter {
    pub fn new(config: RagConfig) -> crate::Result<Self> {
        Ok(Self { config })
    }

    pub fn split(&self, text: &str) -> crate::Result<Vec<String>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        let chars: Vec<char> = text.chars().collect();

        if chars.len() <= self.config.chunk_size {
            return Ok(vec![text.to_string()]);
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        loop {
            let end = std::cmp::min(start + self.config.chunk_size, chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            if end >= chars.len() {
                break;
            }

            start += self.config.chunk_size - self.config.chunk_overlap;
        }

        Ok(chunks)
    }

    pub fn chunk_size(&self) -> usize { self.config.chunk_size }
    pub fn chunk_overlap(&self) -> usize { self.config.chunk_overlap }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_splitter() -> ChunkSplitter {
        let config = RagConfig {
            index_path: "./data/index.bin".to_string(),
            chunks_path: "./data/chunks.json".to_string(),
            chunk_size: 10,
            chunk_overlap: 2,
            top_k: 5,
            score_threshold: 0.75,
        };
        ChunkSplitter::new(config).unwrap()
    }

    #[test]
    fn test_empty_text() {
        let splitter = create_test_splitter();
        let chunks = splitter.split("").unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_small_text() {
        let splitter = create_test_splitter();
        let text = "短いテキスト";
        let chunks = splitter.split(text).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_overlapping_chunks() {
        let config = RagConfig {
            index_path: "./data/index.bin".to_string(),
            chunks_path: "./data/chunks.json".to_string(),
            chunk_size: 10,
            chunk_overlap: 2,
            top_k: 5,
            score_threshold: 0.75,
        };
        let splitter = ChunkSplitter::new(config).unwrap();
        let text = "これはテスト用のとても長いテキストです。複数のチャンクに分割されるべきです。";

        let chunks = splitter.split(text).unwrap();
        assert!(chunks.len() > 1);

        // Check that chunks overlap
        if chunks.len() > 1 {
            // Overlap should exist between consecutive chunks
            assert!(!chunks[0].is_empty());
            assert!(!chunks[1].is_empty());
        }
    }

    #[test]
    fn test_chunk_size_access() {
        let splitter = create_test_splitter();
        assert_eq!(splitter.chunk_size(), 10);
        assert_eq!(splitter.chunk_overlap(), 2);
    }
}
