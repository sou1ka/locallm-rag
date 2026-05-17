//! Application state for rag-ui
//!
//! Holds all runtime resources shared across Tauri commands.
//! Initialized once at startup and shared via tauri::State.

use anyhow::Result;
use rag_core::{
    chunker::ChunkSplitter, conversation::Conversation, embedder::Embedder, llm::LlmClient,
    store::Store,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Configuration for the UI app
/// Mirrors rag-cli config but owned by Tauri state
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppConfig {
    pub rag: rag_core::config::RagConfig,
    pub embedder: rag_core::config::EmbedderConfig,
    pub llm: rag_core::config::LlmConfig,
    pub conversation: rag_core::config::ConversationConfig,
}

impl AppConfig {
    pub fn load(path: &str) -> Result<Self> {
        let config_path = std::path::Path::new(path);
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;
        let mut config: AppConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

        // config.toml のあるディレクトリを基準に相対パスを絶対パスに展開
        let base_dir = config_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .canonicalize()
            .unwrap_or_else(|_| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });

        config.resolve_paths(&base_dir);
        Ok(config)
    }

    /// 相対パスを base_dir 基準の絶対パスに変換
    fn resolve_paths(&mut self, base_dir: &std::path::Path) {
        self.rag.db_path = resolve(base_dir, &self.rag.db_path);
        self.embedder.onnx_path = resolve(base_dir, &self.embedder.onnx_path);
        self.embedder.tokenizer_path = resolve(base_dir, &self.embedder.tokenizer_path);
        self.conversation.summary_dir = resolve(base_dir, &self.conversation.summary_dir);
        self.conversation.history_dir = resolve(base_dir, &self.conversation.history_dir);
    }
}

/// 相対パスを base_dir 基準で絶対パスに変換するヘルパー
fn resolve(base_dir: &std::path::Path, path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        path.to_string()
    } else {
        base_dir.join(p).to_string_lossy().to_string()
    }
}

/// Inner state (wrapped in Mutex for Tauri State)
pub struct AppStateInner {
    pub store: Store,
    pub embedder: Embedder,
    pub chunker: ChunkSplitter,
    pub llm: LlmClient,
    pub sessions: HashMap<String, Conversation>,
    pub config: AppConfig,
}

impl AppStateInner {
    pub fn init(config: AppConfig) -> Result<Self> {
        let embedder = Embedder::new(config.embedder.clone())
            .map_err(|e| anyhow::anyhow!("Failed to init embedder: {}", e))?;

        let store = Store::open(config.rag.clone(), &config.rag.db_path)
            .map_err(|e| anyhow::anyhow!("Failed to open store: {}", e))?;

        let chunker = ChunkSplitter::new(config.rag.clone())
            .map_err(|e| anyhow::anyhow!("Failed to init chunker: {}", e))?;

        let llm = LlmClient::new(config.llm.clone())
            .map_err(|e| anyhow::anyhow!("Failed to init LLM: {}", e))?;

        Ok(Self {
            store,
            embedder,
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

    /// List all session IDs and titles
    pub fn list_sessions(&self) -> Vec<(String, Option<String>)> {
        self.sessions
            .values()
            .map(|c| (c.id.clone(), c.title.clone()))
            .collect()
    }

    /// Delete a session
    pub fn delete_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some()
    }
}

/// Tauri managed state wrapper
pub struct AppState(pub Mutex<AppStateInner>);

impl AppState {
    pub fn new(inner: AppStateInner) -> Self {
        Self(Mutex::new(inner))
    }
}
