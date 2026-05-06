//! RPC interface definition for rag-daemon
//!
//! Defines the tarpc service trait that CLI and Tauri clients use.
//! All methods are async and return Results.

use serde::{Deserialize, Serialize};

/// Ingest statistics returned after ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestStats {
    pub total_chunks: usize,
    pub files_processed: usize,
    pub files_skipped: usize,
    pub source_path: String,
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_chunks: usize,
    pub by_source_type: Vec<(String, usize)>, // (source_type, count)
    pub index_path: String,
    pub chunks_path: String,
}

/// Query result from RAG pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub answer: String,
    pub contexts: Vec<ContextItem>, // 使用されたRAGチャンク
}

/// Single context item used in RAG response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub content: String,
    pub source_path: String,
    pub score: f32,
}

/// Chat result from conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    pub answer: String,
    pub session_id: String,
    pub turn_count: usize,
    pub summary_compressed: bool, // このターンで要約圧縮が発生したか
}

/// Session info for listing conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub title: Option<String>,
    pub turn_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Daemon status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub model_loaded: bool,
    pub index_loaded: bool,
    pub total_chunks: usize,
    pub version: String,
}

/// tarpc service definition
///
/// This trait is the contract between daemon (server) and clients (CLI/Tauri).
/// tarpc generates client and server stubs from this trait automatically.
#[tarpc::service]
pub trait RagService {
    /// Ingest files from a directory or single file path
    async fn ingest(path: String, extensions: Option<Vec<String>>) -> Result<IngestStats, String>;

    /// Query the RAG pipeline (search + LLM answer)
    async fn query(
        text: String,
        use_rag: bool,       // falseならRAGバイパスでLLM直打ち
        show_context: bool,  // trueならcontextsを返す
        top_k: Option<usize>,
    ) -> Result<QueryResult, String>;

    /// Send a message in a conversation session
    async fn chat(
        session_id: String,
        message: String,
        use_rag: bool,
    ) -> Result<ChatResult, String>;

    /// List all conversation sessions
    async fn list_sessions() -> Result<Vec<SessionInfo>, String>;

    /// Delete a conversation session
    async fn delete_session(session_id: String) -> Result<(), String>;

    /// Get index statistics
    async fn index_stats() -> Result<IndexStats, String>;

    /// Reset (clear) the entire index
    async fn reset_index() -> Result<(), String>;

    /// Reset index for a specific source path only
    async fn reset_index_source(source_path: String) -> Result<(), String>;

    /// Get daemon status
    async fn status() -> Result<DaemonStatus, String>;
}
