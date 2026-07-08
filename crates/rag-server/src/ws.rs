//! WebSocket endpoint: chat + segment単位のTTS/感情配信
//!
//! TTS・感情抽出・キャラクターはオプション機能。取得できなくてもテキスト応答とセッション保存は継続する。

use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use rag_core::{
    conversation::{build_context, Conversation, Message as ConvMessage},
    history::{current_datetime, HistoryManager},
    llm::{build_rag_prompt, datetime_info, default_system_prompt},
    retriever::Retriever,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tts_emotion::{CharacterStore, EmotionRules, SegmentBuffer, TtsClient};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

#[derive(Debug, Deserialize)]
struct WsChatRequest {
    #[serde(rename = "type")]
    msg_type: String,
    text: String,
    session_id: Option<String>,
    #[serde(default = "default_temperature")]
    temperature: f32,
    #[serde(default = "default_max_tokens")]
    max_tokens: u16,
    /// 使用するキャラID（chara/{id}.toml）。省略時はデフォルト動作
    chara: Option<String>,
}

fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u16 {
    2048
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) => match serde_json::from_str::<WsChatRequest>(&text) {
                Ok(req) => {
                    if req.msg_type == "chat" {
                        if let Err(e) = handle_chat(&mut socket, &state, req).await {
                            if send_error(&mut socket, &e.to_string()).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    if send_error(&mut socket, &format!("invalid message: {}", e))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            },
            Some(Ok(Message::Close(_))) => break,
            Some(Ok(_)) => {}
            Some(Err(_)) => break,
            None => break,
        }
    }
}

async fn handle_chat(
    socket: &mut WebSocket,
    state: &AppState,
    req: WsChatRequest,
) -> anyhow::Result<()> {
    // セッション準備（ロック外でIO）
    let session_id = req.session_id.clone().unwrap_or_else(current_datetime);

    let history_dir = {
        let inner = state.0.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
        inner.history_dir.clone()
    };

    let conversation = {
        let history = HistoryManager::new(&history_dir)?;
        if req.session_id.is_some() {
            history
                .find_path(&session_id)
                .and_then(|p| history.load(&p).ok())
                .unwrap_or_else(|| Conversation::new(session_id.clone()))
        } else {
            Conversation::new(session_id.clone())
        }
    };

    // RAG（ロック内。awaitをまたがない）
    let (context, llm, tts, emotion_rules, characters, default_chara) = {
        let inner = state.0.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?;

        let query_embedding = inner.embedder.embed_query(&req.text)?;
        let retriever = Retriever::new(&inner.store, inner.config.rag.clone());
        let rag_results = retriever.retrieve(&query_embedding).unwrap_or_default();
        let context = build_context(&rag_results);

        let llm = inner.llm.clone();
        let tts = inner.tts.clone();
        let emotion_rules = inner.emotion_rules.clone();
        let characters: Option<Arc<CharacterStore>> = inner.characters.clone();
        let default_chara = inner.config.chara.as_ref().and_then(|c| c.default.clone());

        (context, llm, tts, emotion_rules, characters, default_chara)
    };

    // キャラ解決（ロック外）。リクエスト未指定時は config.toml の [chara] default を使う
    let chara_id = req.chara.clone().or(default_chara);
    let character = match (&chara_id, &characters) {
        (Some(id), Some(store)) => {
            let found = store.get(id);
            if found.is_none() {
                tracing::warn!("Character not found: {}", id);
            }
            found
        }
        (Some(id), None) => {
            tracing::warn!("Character '{}' requested but chara dir is not configured", id);
            None
        }
        (None, _) => None,
    };

    let system_prompt = match character {
        Some(c) => format!("{}\n{}", c.system_prompt.trim_end(), datetime_info()),
        None => default_system_prompt(),
    };

    let seg_rules: Option<&EmotionRules> = character
        .and_then(|c| c.emotion.as_ref())
        .or(emotion_rules.as_deref());
    let voice: Option<&str> = character.and_then(|c| c.voice.as_deref());

    let messages = build_rag_prompt(
        &system_prompt,
        &context,
        conversation.to_chat_messages(),
        &req.text,
    );

    let text_preview: String = req.text.chars().take(50).collect();
    tracing::info!(
        "WS chat received: session={} chara={} text=\"{}\"",
        session_id,
        chara_id.as_deref().unwrap_or("-"),
        text_preview
    );

    send_json(
        socket,
        &serde_json::json!({ "type": "session", "session_id": session_id }),
    )
    .await?;

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        let _ = llm
            .complete_stream(messages, req.temperature, req.max_tokens, move |token| {
                let _ = tx.send(token);
            })
            .await;
    });

    let mut seg_buf = SegmentBuffer::new();
    let mut full_response = String::new();
    let mut segment_count = 0usize;

    while let Some(token) = rx.recv().await {
        full_response.push_str(&token);
        for segment in seg_buf.push(&token) {
            send_segment(socket, &tts, seg_rules, voice, &segment).await?;
            segment_count += 1;
        }
    }

    for segment in seg_buf.finish() {
        send_segment(socket, &tts, seg_rules, voice, &segment).await?;
        segment_count += 1;
    }

    // 履歴保存（失敗してもレスポンス配信は妨げない）
    let response_chars = full_response.chars().count();
    let mut conv = conversation;
    conv.push(ConvMessage::user(req.text));
    conv.push(ConvMessage::assistant(full_response));
    match HistoryManager::new(&history_dir) {
        Ok(history) => {
            if let Err(e) = history.save(&conv) {
                tracing::warn!("Failed to save history: {}", e);
            }
        }
        Err(e) => tracing::warn!("Failed to open history manager: {}", e),
    }

    tracing::info!(
        "WS chat done: session={} segments={} response_chars={}",
        session_id,
        segment_count,
        response_chars
    );

    send_json(
        socket,
        &serde_json::json!({ "type": "done", "session_id": session_id }),
    )
    .await?;

    Ok(())
}

/// セグメント1件分の感情抽出・TTS合成・送信
///
/// 感情抽出・TTSの失敗はログのみで処理を継続する（オプション機能のため）
async fn send_segment(
    socket: &mut WebSocket,
    tts: &Option<Arc<TtsClient>>,
    emotion_rules: Option<&EmotionRules>,
    voice: Option<&str>,
    text: &str,
) -> anyhow::Result<()> {
    let (emotion, motion_id) = match emotion_rules {
        Some(rules) => {
            let emotion = rules.extract(text);
            (Some(emotion.name), emotion.motion_id)
        }
        None => (None, None),
    };

    let audio = match tts {
        Some(client) => match client.synthesize_with_voice(text, voice).await {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                tracing::warn!("TTS synthesis failed: {}", e);
                None
            }
        },
        None => None,
    };

    send_json(
        socket,
        &serde_json::json!({
            "type": "segment",
            "text": text,
            "emotion": emotion,
            "motion_id": motion_id,
            "audio": audio.is_some(),
        }),
    )
    .await?;

    if let Some(bytes) = audio {
        socket.send(Message::Binary(bytes)).await?;
    }

    Ok(())
}

async fn send_json(socket: &mut WebSocket, value: &serde_json::Value) -> anyhow::Result<()> {
    socket.send(Message::Text(value.to_string())).await?;
    Ok(())
}

async fn send_error(socket: &mut WebSocket, message: &str) -> Result<(), axum::Error> {
    socket
        .send(Message::Text(
            serde_json::json!({ "type": "error", "message": message }).to_string(),
        ))
        .await
}
