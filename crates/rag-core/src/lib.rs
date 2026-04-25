//! RAG Core Engine
//!
//! A Rust-based RAG (Retrieval-Augmented Generation) engine for local LLM (Ollama).
//! Provides document ingestion, embeddings, retrieval, and conversation management.

pub mod config;
pub mod ingestor;
pub mod chunker;
pub mod embedder;
pub mod store;
pub mod retriever;
pub mod llm;
pub mod conversation;

// Re-export main types
pub use config::Config;
pub use config::{
    RagConfig, EmbedderConfig, OcrConfig, LlmConfig, ConversationConfig, DaemonConfig,
    SourceConfig, AudioConfig, VideoConfig, ImageConfig,
};

// Re-export error types
pub use anyhow::{anyhow, Result};