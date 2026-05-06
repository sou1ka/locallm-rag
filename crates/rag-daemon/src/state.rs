//! Daemon application state
//!
//! Holds all runtime resources: embedder, store, chunker, LLM client, and sessions.
//! Initialized once at daemon startup and shared across all RPC requests.

use crate::config::Config;
use rag_core::{
    chunker::ChunkSplitter,
    conversation::Conversation,
    embedder::Embedder,
    llm::LlmClient,
    store::Store,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state
///
/// Wrapped in Arc<RwLock<>> so it can be shared across async tarpc handlers.
/// RwLock allows multiple concurrent reads (queries) with exclusive writes (ingest).
pub struct AppState {
    pub embedder: Embedder,
    pub store: Store,
    pub chunker: ChunkSplitter,
    pub llm: LlmClient,
    pub sessions: HashMap<String, Conversation>,
    pub config: Config,
}

impl AppState {
    /// Initialize AppState from config
    ///
    /// Loads embedder model and index from disk if they exist.
    pub fn init(config: Config) -> anyhow::Result<Self> {
        // Embedder初期化（ONNXモデルロード）
        let embedder = Embedder::new(config.embedder.clone())
            .map_err(|e| anyhow::anyhow!("Failed to init embedder: {}", e))?;

        // Store初期化（インデックスが存在すればロード、なければ新規作成）
        let store = {
            let index_path = &config.rag.index_path;
            let chunks_path = &config.rag.chunks_path;

            if std::path::Path::new(chunks_path).exists() {
                Store::load(config.rag.clone(), index_path, chunks_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load store: {}", e))?
            } else {
                Store::new(config.rag.clone(), index_path, chunks_path)
                    .map_err(|e| anyhow::anyhow!("Failed to create store: {}", e))?
            }
        };

        // ChunkSplitter初期化
        let chunker = ChunkSplitter::new(config.rag.clone())
            .map_err(|e| anyhow::anyhow!("Failed to init chunker: {}", e))?;

        // LLMクライアント初期化
        let llm = LlmClient::new(config.llm.clone())
            .map_err(|e| anyhow::anyhow!("Failed to init LLM client: {}", e))?;

        Ok(Self {
            embedder,
            store,
            chunker,
            llm,
            sessions: HashMap::new(),
            config,
        })
    }

    /// Get or create a conversation session
    pub fn get_or_create_session(&mut self, session_id: &str) -> &mut Conversation {
        self.sessions
            .entry(session_id.to_string())
            .or_insert_with(|| Conversation::new(session_id.to_string()))
    }

    /// Delete a session
    pub fn delete_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<&Conversation> {
        self.sessions.values().collect()
    }

    /// Total chunk count
    pub fn total_chunks(&self) -> usize {
        self.store.len()
    }

    /// Index stats by source type
    pub fn stats_by_source_type(&self) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for chunk in self.store.chunks() {
            *counts.entry(chunk.source_type.clone()).or_insert(0) += 1;
        }
        let mut result: Vec<(String, usize)> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1)); // 多い順にソート
        result
    }
}

/// Thread-safe shared state wrapper
pub type SharedState = Arc<RwLock<AppState>>;

/// Create shared state from config
pub fn create_shared_state(config: Config) -> anyhow::Result<SharedState> {
    let state = AppState::init(config)?;
    Ok(Arc::new(RwLock::new(state)))
}
