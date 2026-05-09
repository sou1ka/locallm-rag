//! Tauri commands for rag-ui
//!
//! All commands are async and return Result<T, String>.
//! Errors are converted to String for serialization to frontend.

use crate::state::AppState;
use rag_core::{
    conversation::{build_context, Conversation, ConversationManager},
    retriever::Retriever,
};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

/// Ingest stats returned to frontend
#[derive(Debug, Serialize)]
pub struct IngestResult {
    pub total_chunks: usize,
    pub files_processed: usize,
}

/// Session info for sidebar list
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub title: Option<String>,
    pub turn_count: usize,
}

/// Single message for history display
#[derive(Debug, Serialize)]
pub struct MessageInfo {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

/// Index statistics
#[derive(Debug, Serialize)]
pub struct IndexStats {
    pub total_chunks: usize,
    pub by_source_type: Vec<(String, usize)>,
}

/// History entry info for frontend
#[derive(Debug, Serialize)]
pub struct HistoryEntryInfo {
    pub session_id: String,
    pub title: String,
    pub path: String,
}

/// Ingest files from a path
#[tauri::command]
pub async fn ingest(
    path: String,
    extensions: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<IngestResult, String> {
    let target = std::path::Path::new(&path);
    if !target.exists() {
        return Err(format!("Path not found: {}", path));
    }

    let (config_rag, config_embedder) = {
        let inner = state.0.lock().unwrap();
        (inner.config.rag.clone(), inner.config.embedder.clone())
    };

    // インジェスト処理（ロックの外で実行）
    let chunker = rag_core::chunker::ChunkSplitter::new(config_rag.clone())
        .map_err(|e| e.to_string())?;

    let embedder = rag_core::embedder::Embedder::new(config_embedder)
        .map_err(|e| e.to_string())?;

    let chunks = if target.is_dir() {
        rag_core::ingestor::ingest_directory(
            target,
            extensions.as_deref(),
            &config_rag,
        )
        .map_err(|e| e.to_string())?
    } else {
        rag_core::ingestor::ingest_file(target, &config_rag)
            .map_err(|e| e.to_string())?
    };

    let files_processed = chunks
        .iter()
        .map(|c| c.file_path.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();

    let total_chunks = chunks.len();

    // バッチEmbedding（ロックの外で実行）
    let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = embedder
        .embed_documents(texts)
        .map_err(|e| e.to_string())?;

    // Storeへの追加（ロックを取得して書き込み）
    {
        let mut inner = state.0.lock().unwrap();
        for (mut chunk, embedding) in chunks.into_iter().zip(embeddings) {
            chunk.id = inner.store.len();
            inner
                .store
                .add_chunk(embedding, chunk)
                .map_err(|e| e.to_string())?;
        }
        inner.store.save().map_err(|e| e.to_string())?;
    }

    Ok(IngestResult {
        total_chunks,
        files_processed,
    })
}

/// Query the RAG pipeline (single shot, no session)
#[tauri::command]
pub async fn query(
    text: String,
    use_rag: bool,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let (store_snapshot, config_rag, config_embedder, config_llm) = {
        let inner = state.0.lock().unwrap();
        (
            inner.store.chunks().to_vec(),
            inner.config.rag.clone(),
            inner.config.embedder.clone(),
            inner.config.llm.clone(),
        )
    };

    let embedder = rag_core::embedder::Embedder::new(config_embedder)
        .map_err(|e| e.to_string())?;

    let llm = rag_core::llm::LlmClient::new(config_llm)
        .map_err(|e| e.to_string())?;

    // RAG検索
    let context = if use_rag && !store_snapshot.is_empty() {
        let query_embedding = embedder
            .embed_query(&text)
            .map_err(|e| e.to_string())?;

        let store = rag_core::store::Store::open(config_rag.clone(), &config_rag.db_path)
            .map_err(|e| e.to_string())?;

        let retriever = Retriever::new(&store, config_rag);
        let results = retriever
            .retrieve(&query_embedding)
            .map_err(|e| e.to_string())?;

        build_context(&results)
    } else {
        String::new()
    };

    // LLM呼び出し
    let messages = rag_core::llm::build_rag_prompt(
        rag_core::llm::default_system_prompt(),
        &context,
        vec![],
        &text,
    );

    let response = llm
        .complete(messages, 0.7, 2048)
        .await
        .map_err(|e| e.to_string())?;

    Ok(response)
}

/// Send a message in a chat session
#[tauri::command]
pub async fn chat(
    session_id: String,
    message: String,
    use_rag: bool,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let (config_rag, config_embedder, config_llm, config_conv) = {
        let inner = state.0.lock().unwrap();
        (
            inner.config.rag.clone(),
            inner.config.embedder.clone(),
            inner.config.llm.clone(),
            inner.config.conversation.clone(),
        )
    };

    let embedder = rag_core::embedder::Embedder::new(config_embedder)
        .map_err(|e| e.to_string())?;

    let llm = rag_core::llm::LlmClient::new(config_llm)
        .map_err(|e| e.to_string())?;

    // RAG検索
    let rag_results = if use_rag {
        let store = rag_core::store::Store::open(config_rag.clone(), &config_rag.db_path)
            .unwrap_or_else(|e| panic!("Failed to open store: {}", e));

        if !store.is_empty() {
            let query_embedding = embedder
                .embed_query(&message)
                .map_err(|e| e.to_string())?;
            let retriever = Retriever::new(&store, config_rag);
            retriever
                .retrieve(&query_embedding)
                .map_err(|e| e.to_string())?
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // セッション取得
    let conversation = {
        let mut inner = state.0.lock().unwrap();
        inner
            .get_or_create_session(&session_id)
            .clone()
    };

    let mut manager = ConversationManager::new(conversation, config_conv, llm);

    // ストリーミングでフロントへ送信
    let app_handle_clone = app_handle.clone();
    let session_id_clone = session_id.clone();

    let response = manager
        .chat_stream(
            &message,
            &rag_results,
            rag_core::llm::default_system_prompt(),
            move |token| {
                app_handle_clone
                    .emit_all(
                        "stream_token",
                        StreamPayload {
                            session_id: session_id_clone.clone(),
                            token,
                        },
                    )
                    .ok();
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    // セッションを保存
    {
        let mut inner = state.0.lock().unwrap();
        inner
            .sessions
            .insert(session_id.clone(), manager.conversation.clone());
    }

    Ok(response)
}

/// List all sessions
#[tauri::command]
pub fn list_sessions(state: State<'_, AppState>) -> Vec<SessionInfo> {
    let inner = state.0.lock().unwrap();
    inner
        .sessions
        .values()
        .map(|c| SessionInfo {
            session_id: c.id.clone(),
            title: c.title.clone(),
            turn_count: c.turn_count(),
        })
        .collect()
}

/// Get messages for a session
#[tauri::command]
pub fn get_messages(
    session_id: String,
    state: State<'_, AppState>,
) -> Vec<MessageInfo> {
    let inner = state.0.lock().unwrap();
    match inner.sessions.get(&session_id) {
        Some(conv) => conv
            .messages
            .iter()
            .map(|m| MessageInfo {
                role: m.role.clone(),
                content: m.content.clone(),
                created_at: m.created_at.clone(),
            })
            .collect(),
        None => vec![],
    }
}

/// Delete a session
#[tauri::command]
pub fn delete_session(
    session_id: String,
    state: State<'_, AppState>,
) -> bool {
    let mut inner = state.0.lock().unwrap();
    inner.delete_session(&session_id)
}

/// Get index statistics
#[tauri::command]
pub fn index_stats(state: State<'_, AppState>) -> IndexStats {
    let inner = state.0.lock().unwrap();
    let mut by_type: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for chunk in inner.store.chunks() {
        *by_type.entry(chunk.source_type.clone()).or_insert(0) += 1;
    }
    let mut by_source_type: Vec<(String, usize)> =
        by_type.into_iter().collect();
    by_source_type.sort_by(|a, b| b.1.cmp(&a.1));

    IndexStats {
        total_chunks: inner.store.len(),
        by_source_type,
    }
}

/// Reset the index
#[tauri::command]
pub fn reset_index(state: State<'_, AppState>) -> Result<(), String> {
    let mut inner = state.0.lock().unwrap();
    inner.store.clear().map_err(|e| e.to_string())?;
    Ok(())
}

/// Streaming token payload
#[derive(Clone, Serialize)]
struct StreamPayload {
    session_id: String,
    token: String,
}

/// Load conversation history list from history/ directory
#[tauri::command]
pub fn load_history(state: State<'_, AppState>) -> Vec<HistoryEntryInfo> {
    let inner = state.0.lock().unwrap();
    let history = match rag_core::history::HistoryManager::new(
        &inner.config.conversation.history_dir
    ) {
        Ok(h) => h,
        Err(_) => return vec![],
    };

    match history.list() {
        Ok(entries) => entries
            .into_iter()
            .map(|e| HistoryEntryInfo {
                session_id: e.session_id,
                title: e.title,
                path: e.path.to_string_lossy().to_string(),
            })
            .collect(),
        Err(_) => vec![],
    }
}

/// Load a specific conversation from history/
#[tauri::command]
pub fn load_conversation(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<rag_core::conversation::Conversation, String> {
    let mut inner = state.0.lock().unwrap();
    let history = rag_core::history::HistoryManager::new(
        &inner.config.conversation.history_dir
    )
    .map_err(|e| e.to_string())?;

    match history.find_path(&session_id) {
        Some(path) => {
            let conversation = history.load(&path).map_err(|e| e.to_string())?;
            // セッション状態に保持しないと次の発言時に新規会話として扱われる
            inner.sessions.insert(session_id, conversation.clone());
            Ok(conversation)
        }
        None => Err(format!("Session not found: {}", session_id)),
    }
}

/// Save current session to history
#[tauri::command]
pub fn save_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let inner = state.0.lock().unwrap();
    let history = rag_core::history::HistoryManager::new(
        &inner.config.conversation.history_dir,
    )
    .map_err(|e| e.to_string())?;

    if let Some(conv) = inner.sessions.get(&session_id) {
        if !conv.messages.is_empty() {
            history.save(conv).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Save all active sessions to history
#[tauri::command]
pub fn save_all_sessions(state: State<'_, AppState>) -> Result<(), String> {
    let inner = state.0.lock().unwrap();
    let history = rag_core::history::HistoryManager::new(
        &inner.config.conversation.history_dir,
    )
    .map_err(|e| e.to_string())?;

    for conv in inner.sessions.values() {
        if !conv.messages.is_empty() {
            if let Err(e) = history.save(conv) {
                eprintln!("Failed to save session {}: {}", conv.id, e);
            }
        }
    }
    Ok(())
}
