// lib/stores/chat.js
// Svelte stores for chat state management

import { writable, derived, get } from 'svelte/store';
import * as api from '../tauri.js';

// ─── セッション一覧 ───────────────────────────────────────────
export const sessions = writable([]);
export const activeSession = writable(null); // 現在選択中のsession_id

// ─── メッセージ ───────────────────────────────────────────────
export const messages = writable([]);

// ─── UI状態 ──────────────────────────────────────────────────
export const isLoading = writable(false);      // LLM応答待ち
export const isIngesting = writable(false);    // インジェスト中
export const streamingText = writable('');     // ストリーミング中のテキスト
export const useRag = writable(true);          // RAG有効/無効
export const errorMessage = writable('');      // エラーメッセージ

// ─── インデックス統計 ─────────────────────────────────────────
export const indexStats = writable({
  total_chunks: 0,
  by_source_type: [],
});

// ─── アクション ───────────────────────────────────────────────

/**
 * history/ からセッション一覧を読み込む
 */
export async function loadSessions() {
  try {
    const entries = await api.loadHistory();
    sessions.set(entries);
  } catch (e) {
    setError(`Failed to load sessions: ${e}`);
  }
}

/**
 * セッションを選択してメッセージを読み込む
 * @param {string} sessionId
 */
 // 別の会話を選択する前に現在の会話を保存
 export async function selectSession(sessionId) {
   // 現在のセッションを保存
   const current = get(activeSession);
   if (current) {
     try {
       await api.saveSession(current);
     } catch (e) {
       console.error('Failed to save session:', e);
     }
   }

   try {
     activeSession.set(sessionId);
     const conversation = await api.loadConversation(sessionId);
     messages.set(
       conversation.messages.map((m) => ({
         role: m.role,
         content: m.content,
         created_at: m.created_at,
       }))
     );
   } catch (e) {
     setError(`Failed to load conversation: ${e}`);
     messages.set([]);
   }
 }

 // 新規会話開始前に現在の会話を保存
 export async function newSession() {  // ← async に変更
   const current = get(activeSession);
   if (current) {
     try {
       await api.saveSession(current);
       await loadSessions();
     } catch (e) {
       console.error('Failed to save session:', e);
     }
   }
   activeSession.set(generateSessionId());
   messages.set([]);
   streamingText.set('');
   errorMessage.set('');
 }

/**
 * メッセージを送信してLLMから応答を受け取る
 * @param {string} text - ユーザーメッセージ
 */
export async function sendMessage(text) {
  if (!text.trim()) return;

  const sessionId = get(activeSession) || generateSessionId();
  activeSession.set(sessionId);
  const ragEnabled = get(useRag);

  // ユーザーメッセージを追加
  messages.update((msgs) => [
    ...msgs,
    {
      role: 'user',
      content: text,
      created_at: new Date().toISOString(),
    },
  ]);

  // ストリーミング開始
  isLoading.set(true);
  streamingText.set('');
  errorMessage.set('');

  // アシスタントのプレースホルダーを追加
  messages.update((msgs) => [
    ...msgs,
    {
      role: 'assistant',
      content: '',
      created_at: new Date().toISOString(),
      streaming: true,
    },
  ]);

  try {
    await api.chat(sessionId, text, ragEnabled, (token) => {
      // ストリーミングトークンを最後のメッセージに追記
      streamingText.update((t) => t + token);
      messages.update((msgs) => {
        const updated = [...msgs];
        const last = updated[updated.length - 1];
        if (last && last.streaming) {
          last.content += token;
        }
        return updated;
      });
    });

    // ストリーミング完了
    messages.update((msgs) => {
      const updated = [...msgs];
      const last = updated[updated.length - 1];
      if (last && last.streaming) {
        delete last.streaming;
      }
      return updated;
    });

    // セッション一覧を更新（タイトルが生成されている可能性があるため）
    await loadSessions();
  } catch (e) {
    setError(`Failed to send message: ${e}`);
    // エラー時はプレースホルダーを削除
    messages.update((msgs) => msgs.filter((m) => !m.streaming));
  } finally {
    isLoading.set(false);
    streamingText.set('');
  }
}

/**
 * ファイル/ディレクトリをインジェスト
 * @param {string} path
 * @param {string[]|null} extensions
 */
export async function ingestPath(path, extensions = null) {
  isIngesting.set(true);
  errorMessage.set('');

  try {
    const result = await api.ingest(path, extensions);
    await refreshIndexStats();
    return result;
  } catch (e) {
    setError(`Ingest failed: ${e}`);
    throw e;
  } finally {
    isIngesting.set(false);
  }
}

/**
 * インデックス統計を更新
 */
export async function refreshIndexStats() {
  try {
    const stats = await api.indexStats();
    indexStats.set(stats);
  } catch (e) {
    console.error('Failed to load index stats:', e);
  }
}

/**
 * インデックスをリセット
 */
export async function resetIndex() {
  try {
    await api.resetIndex();
    await refreshIndexStats();
  } catch (e) {
    setError(`Reset failed: ${e}`);
    throw e;
  }
}

/**
 * セッションを削除
 * @param {string} sessionId
 */
export async function deleteSession(sessionId) {
  try {
    await api.deleteSession(sessionId);
    await loadSessions();

    // 削除したセッションが選択中だった場合は新規セッションへ
    if (get(activeSession) === sessionId) {
      newSession();
    }
  } catch (e) {
    setError(`Failed to delete session: ${e}`);
  }
}

export async function saveAllSessions() {
  try {
    await api.saveAllSessions();
  } catch (e) {
    console.error('Failed to save all sessions:', e);
  }
}

// ─── ユーティリティ ───────────────────────────────────────────

/**
 * エラーメッセージをセット（5秒後に自動クリア）
 * @param {string} msg
 */
function setError(msg) {
  errorMessage.set(msg);
  setTimeout(() => errorMessage.set(''), 5000);
}

/**
 * セッションIDを生成（YYYYMMDD_HHMMSS形式）
 * CLIのnew_session_id()と同じ形式
 */
function generateSessionId() {
  const now = new Date();
  const pad = (n) => String(n).padStart(2, '0');
  return (
    `${now.getFullYear()}` +
    `${pad(now.getMonth() + 1)}` +
    `${pad(now.getDate())}` +
    `_` +
    `${pad(now.getHours())}` +
    `${pad(now.getMinutes())}` +
    `${pad(now.getSeconds())}`
  );
}
