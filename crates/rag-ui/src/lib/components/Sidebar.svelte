<!-- lib/components/Sidebar.svelte -->
<!-- Conversation history sidebar with session list -->

<script>
  import {
    sessions,
    activeSession,
    isLoading,
    loadSessions,
    selectSession,
    deleteSession,
  } from '../stores/chat.js';
  import { onMount } from 'svelte';

  onMount(async () => {
    await loadSessions();
  });

  async function handleSelectSession(sessionId) {
    if ($isLoading) return;
    await selectSession(sessionId);
  }

  async function handleDeleteSession(e, sessionId) {
    e.stopPropagation();
    if (!confirm('この会話を削除しますか？')) return;
    await deleteSession(sessionId);
  }
</script>

<aside class="sidebar">
  <!-- セッション一覧 -->
  <div class="session-list">
    {#if $sessions.length === 0}
      <div class="empty-state">
        <p>会話履歴がありません</p>
        <p class="hint">新規会話を開始してください</p>
      </div>
    {:else}
      {#each $sessions as session (session.session_id)}
        <div
          class="session-item"
          class:active={$activeSession === session.session_id}
          on:click={() => handleSelectSession(session.session_id)}
          role="button"
          tabindex="0"
          on:keydown={(e) => e.key === 'Enter' && handleSelectSession(session.session_id)}
        >
          <div class="session-info">
            <span class="session-title">
              {session.title || '無題の会話'}
            </span>
            <span class="session-id">
              {formatSessionId(session.session_id)}
            </span>
          </div>
          <button
            class="delete-btn"
            on:click={(e) => handleDeleteSession(e, session.session_id)}
            title="削除"
            aria-label="会話を削除"
          >
            ✕
          </button>
        </div>
      {/each}
    {/if}
  </div>
</aside>

<script context="module">
  /**
   * YYYYMMDD_HHMMSS → YYYY/MM/DD HH:MM に変換
   * @param {string} sessionId
   */
  export function formatSessionId(sessionId) {
    if (!sessionId || sessionId.length < 15) return sessionId;
    const y = sessionId.slice(0, 4);
    const mo = sessionId.slice(4, 6);
    const d = sessionId.slice(6, 8);
    const h = sessionId.slice(9, 11);
    const mi = sessionId.slice(11, 13);
    return `${y}/${mo}/${d} ${h}:${mi}`;
  }
</script>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .session-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .empty-state {
    padding: 24px 16px;
    text-align: center;
    color: #6b6b8d;
    font-size: 13px;
  }

  .empty-state .hint {
    margin-top: 4px;
    font-size: 12px;
    opacity: 0.7;
  }

  .session-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    cursor: pointer;
    border-radius: 6px;
    margin: 2px 8px;
    transition: background 0.15s;
    color: #c8c8e0;
  }

  .session-item:hover {
    background: #2d2d4e;
  }

  .session-item.active {
    background: #2d2d4e;
    border-left: 3px solid #4a90d9;
    color: #fff;
  }

  .session-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .session-title {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .session-id {
    font-size: 11px;
    color: #6b6b8d;
  }

  .delete-btn {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    color: #6b6b8d;
    border: none;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s, background 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .session-item:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    background: #c0392b;
    color: #fff;
  }

  /* スクロールバー */
  .session-list::-webkit-scrollbar {
    width: 4px;
  }

  .session-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .session-list::-webkit-scrollbar-thumb {
    background: #3d3d5c;
    border-radius: 2px;
  }
</style>
