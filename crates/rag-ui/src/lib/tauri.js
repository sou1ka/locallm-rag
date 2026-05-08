// lib/tauri.js
// Tauri command wrappers for rag-ui
// All backend communication goes through this module.

import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

/**
 * Ingest files from a path
 * @param {string} path - File or directory path
 * @param {string[]|null} extensions - File extensions to filter (null = all)
 * @returns {Promise<{total_chunks: number, files_processed: number}>}
 */
export async function ingest(path, extensions = null) {
  return await invoke('ingest', { path, extensions });
}

/**
 * Single-shot query (no session)
 * @param {string} text - Query text
 * @param {boolean} useRag - Whether to use RAG
 * @returns {Promise<string>}
 */
export async function query(text, useRag = true) {
  return await invoke('query', { text, useRag });
}

/**
 * Send a message in a chat session (streaming)
 * @param {string} sessionId - Session ID
 * @param {string} message - User message
 * @param {boolean} useRag - Whether to use RAG
 * @param {function} onToken - Callback for each streaming token
 * @returns {Promise<string>} - Full response text
 */
export async function chat(sessionId, message, useRag = true, onToken = null) {
  // ストリーミングトークンのリスナーを設定
  let unlisten = null;
  if (onToken) {
    unlisten = await listen('stream_token', (event) => {
      if (event.payload.session_id === sessionId) {
        onToken(event.payload.token);
      }
    });
  }

  try {
    const response = await invoke('chat', {
      sessionId,
      message,
      useRag,
    });
    return response;
  } finally {
    // リスナーを解除
    if (unlisten) {
      unlisten();
    }
  }
}

/**
 * List all conversation sessions
 * @returns {Promise<Array<{session_id: string, title: string|null, turn_count: number}>>}
 */
export async function listSessions() {
  return await invoke('list_sessions');
}

/**
 * Get messages for a session
 * @param {string} sessionId
 * @returns {Promise<Array<{role: string, content: string, created_at: string}>>}
 */
export async function getMessages(sessionId) {
  return await invoke('get_messages', { sessionId });
}

/**
 * Delete a session
 * @param {string} sessionId
 * @returns {Promise<boolean>}
 */
export async function deleteSession(sessionId) {
  return await invoke('delete_session', { sessionId });
}

/**
 * Get index statistics
 * @returns {Promise<{total_chunks: number, by_source_type: Array<[string, number]>}>}
 */
export async function indexStats() {
  return await invoke('index_stats');
}

/**
 * Reset the entire index
 * @returns {Promise<void>}
 */
export async function resetIndex() {
  return await invoke('reset_index');
}

/**
 * Load conversation history from history/ directory
 * This reads the JSON files directly via Tauri fs API
 * @returns {Promise<Array<{session_id: string, title: string, path: string}>>}
 */
export async function loadHistory() {
  return await invoke('load_history');
}

/**
 * Load a specific conversation by session_id
 * @param {string} sessionId
 * @returns {Promise<{id: string, title: string|null, messages: Array, summary: string|null}>}
 */
export async function loadConversation(sessionId) {
  return await invoke('load_conversation', { sessionId });
}

export async function saveSession(sessionId) {
  return await invoke('save_session', { sessionId });
}

export async function saveAllSessions() {
  return await invoke('save_all_sessions');
}
