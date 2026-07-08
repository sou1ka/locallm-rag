//! HTTP routes for rag-server

use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Json, Path, State},
    http::{header, Request, StatusCode},
    middleware::{self, Next},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, post},
    Router,
};
use futures::stream::{self, StreamExt};
use rag_core::{
    conversation::{Conversation, Message},
    history::{current_datetime, HistoryManager},
    ingestor,
    llm::{build_rag_prompt, default_system_prompt, ChatMessage, MessageRole},
    retriever::Retriever,
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, path::Path as FsPath};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/ingest", post(ingest))
        .route("/v1/index/stats", get(index_stats))
        .route("/v1/index", delete(reset_index))
        .route("/v1/sessions", get(list_sessions))
        .route("/v1/sessions/:id", get(get_session))
        .route("/v1/sessions/:id", delete(delete_session))
        .route("/v1/ws", get(crate::ws::ws_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .route("/v1/health", get(health))
        .merge(protected)
        .with_state(state)
}

// ── Auth ──────────────────────────────────────────────────────────────────────

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = {
        let inner = state.0.lock().unwrap();
        inner.config.server.api_key.clone()
    };

    if api_key.is_empty() {
        return Ok(next.run(req).await);
    }

    let expected = format!("Bearer {}", api_key);
    let provided = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

// ── Health ────────────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// ── Chat completions ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<ApiMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default = "default_temperature")]
    temperature: f32,
    #[serde(default = "default_max_tokens")]
    max_tokens: u16,
    /// 既存セッションを継続する場合に指定。省略時は新規セッションを作成。
    session_id: Option<String>,
}

fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u16 {
    2048
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ApiMessage {
    role: String,
    content: MessageContent,
}

impl ApiMessage {
    fn text(&self) -> String {
        self.content.as_text()
    }
}

/// OpenAI互換の content フィールド（文字列と配列の両形式に対応）
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            // テキスト部分のみ結合して返す（画像等は無視）
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| p.text.as_deref())
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ContentPart {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    image_url: Option<serde_json::Value>,
}

async fn chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if req.messages.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "messages must not be empty" })),
        ));
    }

    let last_user = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "no user message found" })),
            )
        })?
        .text();

    // セッション準備（ロック外でIO）
    let session_id = req.session_id.clone().unwrap_or_else(current_datetime);

    let history_dir = {
        let inner = state.0.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "state lock poisoned" })),
            )
        })?;
        inner.history_dir.clone()
    };

    let conversation = {
        let history = HistoryManager::new(&history_dir).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("history error: {}", e) })),
            )
        })?;
        if req.session_id.is_some() {
            history
                .find_path(&session_id)
                .and_then(|p| history.load(&p).ok())
                .unwrap_or_else(|| Conversation::new(session_id.clone()))
        } else {
            Conversation::new(session_id.clone())
        }
    };

    // RAG + プロンプト構築（ロック内）
    let (messages, llm, model_name) = {
        let inner = state.0.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "state lock poisoned" })),
            )
        })?;

        let query_embedding = inner.embedder.embed_query(&last_user).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("embedding failed: {}", e) })),
            )
        })?;

        let retriever = Retriever::new(&inner.store, inner.config.rag.clone());
        let rag_results = retriever.retrieve(&query_embedding).unwrap_or_default();
        let context = rag_core::conversation::build_context(&rag_results);

        let (system_prompt, history_msgs) = if req.session_id.is_some() {
            (default_system_prompt(), conversation.to_chat_messages())
        } else {
            extract_system_and_history(&req.messages)
        };

        let messages = build_rag_prompt(&system_prompt, &context, history_msgs, &last_user);
        let llm = inner.llm.clone();
        let model_name = inner.config.llm.model.clone();

        (messages, llm, model_name)
    };

    let completion_id = format!("chatcmpl-{}", Uuid::new_v4().simple());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if req.stream {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let id_for_tokens = completion_id.clone();
        let model_for_tokens = model_name.clone();
        let last_user_for_save = last_user.clone();
        let session_id_for_event = session_id.clone();

        tokio::spawn(async move {
            let mut full_response = String::new();
            let _ = llm
                .complete_stream(messages, req.temperature, req.max_tokens, |token| {
                    full_response.push_str(&token);
                    let _ = tx.send(token);
                })
                .await;

            let mut conv = conversation;
            conv.push(Message::user(last_user_for_save));
            conv.push(Message::assistant(full_response));
            if let Ok(history) = HistoryManager::new(&history_dir) {
                let _ = history.save(&conv);
            }
        });

        // 最初に session_id を通知
        let session_event = stream::once(async move {
            Ok::<Event, Infallible>(
                Event::default()
                    .data(serde_json::json!({ "session_id": session_id_for_event }).to_string()),
            )
        });

        let token_stream = UnboundedReceiverStream::new(rx).map(move |token| {
            let chunk = serde_json::json!({
                "id": id_for_tokens,
                "object": "chat.completion.chunk",
                "created": created,
                "model": model_for_tokens,
                "choices": [{"index": 0, "delta": { "content": token }, "finish_reason": null}]
            });
            Ok::<Event, Infallible>(Event::default().data(chunk.to_string()))
        });

        let done_event = stream::once(async move {
            let stop_chunk = serde_json::json!({
                "id": completion_id,
                "object": "chat.completion.chunk",
                "created": created,
                "model": model_name,
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]
            });
            Ok::<Event, Infallible>(Event::default().data(stop_chunk.to_string()))
        });
        let done_marker =
            stream::once(async { Ok::<Event, Infallible>(Event::default().data("[DONE]")) });

        Ok(Sse::new(
            session_event
                .chain(token_stream)
                .chain(done_event)
                .chain(done_marker),
        )
        .into_response())
    } else {
        let response_text = llm
            .complete(messages, req.temperature, req.max_tokens)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("LLM error: {}", e) })),
                )
            })?;

        let mut conv = conversation;
        conv.push(Message::user(last_user));
        conv.push(Message::assistant(response_text.clone()));
        if let Ok(history) = HistoryManager::new(&history_dir) {
            let _ = history.save(&conv);
        }

        let body = serde_json::json!({
            "id": completion_id,
            "object": "chat.completion",
            "created": created,
            "model": model_name,
            "session_id": session_id,
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": response_text },
                "finish_reason": "stop"
            }],
            "usage": null
        });

        Ok(Json(body).into_response())
    }
}

/// Split messages into (system_prompt, history_before_last_user_message)
fn extract_system_and_history(messages: &[ApiMessage]) -> (String, Vec<ChatMessage>) {
    let (start, system_prompt) = if messages.first().map(|m| m.role.as_str()) == Some("system") {
        (1, messages[0].text())
    } else {
        (0, default_system_prompt())
    };

    let rest = &messages[start..];

    let history_end = rest
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == "user")
        .last()
        .map(|(i, _)| i)
        .unwrap_or(rest.len());

    let history = rest[..history_end]
        .iter()
        .filter_map(|m| match m.role.as_str() {
            "user" => Some(ChatMessage::new(MessageRole::User, m.text())),
            "assistant" => Some(ChatMessage::new(MessageRole::Assistant, m.text())),
            _ => None,
        })
        .collect();

    (system_prompt, history)
}

// ── Sessions ──────────────────────────────────────────────────────────────────

async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let history_dir = {
        let inner = state.0.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "state lock poisoned" })),
            )
        })?;
        inner.history_dir.clone()
    };

    let history = HistoryManager::new(&history_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("history error: {}", e) })),
        )
    })?;

    let entries = history.list().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("list error: {}", e) })),
        )
    })?;

    let sessions: Vec<_> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "session_id": e.session_id,
                "title": e.title,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let history_dir = {
        let inner = state.0.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "state lock poisoned" })),
            )
        })?;
        inner.history_dir.clone()
    };

    let history = HistoryManager::new(&history_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("history error: {}", e) })),
        )
    })?;

    let path = history.find_path(&id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("session not found: {}", id) })),
        )
    })?;

    let conv = history.load(&path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("load error: {}", e) })),
        )
    })?;

    let messages: Vec<_> = conv
        .messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content,
                "created_at": m.created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "session_id": conv.id,
        "title": conv.title,
        "messages": messages,
        "created_at": conv.created_at,
        "updated_at": conv.updated_at,
    })))
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let history_dir = {
        let inner = state.0.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "state lock poisoned" })),
            )
        })?;
        inner.history_dir.clone()
    };

    let history = HistoryManager::new(&history_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("history error: {}", e) })),
        )
    })?;

    let deleted = history.delete(&id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("delete error: {}", e) })),
        )
    })?;

    if deleted {
        Ok(Json(
            serde_json::json!({ "status": "ok", "message": format!("session {} deleted", id) }),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("session not found: {}", id) })),
        ))
    }
}

// ── Ingest ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct IngestRequest {
    path: String,
    #[serde(default)]
    extensions: Vec<String>,
}

async fn ingest(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let target = FsPath::new(&req.path);
    if !target.exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("path not found: {}", req.path) })),
        ));
    }

    let mut inner = state.0.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "state lock poisoned" })),
        )
    })?;

    let rag_config = inner.config.rag.clone();
    let extensions: Option<&[String]> = if req.extensions.is_empty() {
        None
    } else {
        Some(&req.extensions)
    };

    let chunks = if target.is_dir() {
        ingestor::ingest_directory(target, extensions, &rag_config)
    } else {
        ingestor::ingest_file(target, &rag_config)
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("ingest failed: {}", e) })),
        )
    })?;

    let chunk_count = chunks.len();

    let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = inner.embedder.embed_documents(texts).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("embedding failed: {}", e) })),
        )
    })?;

    for (mut chunk, embedding) in chunks.into_iter().zip(embeddings) {
        chunk.id = inner.store.len();
        inner.store.add_chunk(embedding, chunk).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("store error: {}", e) })),
            )
        })?;
    }

    inner.store.save().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("save failed: {}", e) })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "chunks_added": chunk_count,
        "total_chunks": inner.store.len()
    })))
}

// ── Index stats ───────────────────────────────────────────────────────────────

async fn index_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let inner = state.0.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "state lock poisoned" })),
        )
    })?;

    Ok(Json(serde_json::json!({
        "total_chunks": inner.store.len(),
        "db_path": inner.config.rag.db_path,
        "model": inner.config.llm.model,
        "top_k": inner.config.rag.top_k,
        "score_threshold": inner.config.rag.score_threshold,
    })))
}

// ── Reset index ───────────────────────────────────────────────────────────────

async fn reset_index(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let mut inner = state.0.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "state lock poisoned" })),
        )
    })?;

    inner.store.clear().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("reset failed: {}", e) })),
        )
    })?;

    Ok(Json(serde_json::json!({ "status": "ok", "message": "index reset" })))
}
