<!-- lib/components/ChatWindow.svelte -->
<!-- Main chat interface with message list and input -->

<script>
  import { onMount, afterUpdate } from 'svelte';
  import MessageBubble from './MessageBubble.svelte';
  import {
    messages,
    isLoading,
    errorMessage,
    useRag,
    activeSession,
    sendMessage,
  } from '../stores/chat.js';

  let inputText = '';
  let messagesEndEl;
  let textareaEl;
  let autoScroll = true;

  // メッセージ更新時に自動スクロール
  afterUpdate(() => {
    if (autoScroll && messagesEndEl) {
      messagesEndEl.scrollIntoView({ behavior: 'smooth' });
    }
  });

  // スクロール位置を監視（手動スクロール時は自動スクロールを止める）
  function handleScroll(e) {
    const el = e.target;
    const threshold = 100;
    autoScroll = el.scrollHeight - el.scrollTop - el.clientHeight < threshold;
  }

  async function handleSubmit() {
    const text = inputText.trim();
    if (!text || $isLoading) return;

    inputText = '';
    adjustTextareaHeight();
    autoScroll = true;

    // フォーカスを維持
    await sendMessage(text);

    // 送信後もフォーカスをtextareaに戻す
    if (textareaEl) textareaEl.focus();
  }

  function handleKeydown(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
      // フォーカスを維持
      if (textareaEl) textareaEl.focus();
    }
  }

  function adjustTextareaHeight() {
    if (!textareaEl) return;
    textareaEl.style.height = 'auto';
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, 160) + 'px';
  }

  onMount(() => {
    if (textareaEl) textareaEl.focus();
  });
</script>

<div class="chat-window">
  <!-- ツールバー -->
  <div class="toolbar">
    <div class="toolbar-left">
      {#if $activeSession}
        <span class="session-badge">{$activeSession}</span>
      {:else}
        <span class="session-badge dimmed">新規会話</span>
      {/if}
    </div>
    <div class="toolbar-right">
      <label class="rag-toggle">
        <input
          type="checkbox"
          bind:checked={$useRag}
          disabled={$isLoading}
        />
        <span class="toggle-label">RAG</span>
      </label>
    </div>
  </div>

  <!-- メッセージ一覧 -->
  <div class="messages-area" on:scroll={handleScroll}>
    {#if $messages.length === 0}
      <div class="welcome">
        <div class="welcome-icon">🤖</div>
        <h2>LOCALLM_RAG</h2>
        <p>ローカルLLM + RAGエンジン</p>
        <ul class="hints">
          <li>左のサイドバーから過去の会話を選択できます</li>
          <li>RAGをONにすると、インデックスされたドキュメントを参照して回答します</li>
          <li>インデックス構築はサイドバー下部の設定から行えます</li>
        </ul>
      </div>
    {:else}
      {#each $messages as msg, i (i)}
        <MessageBubble
          role={msg.role}
          content={msg.content}
          createdAt={msg.created_at}
          streaming={msg.streaming || false}
        />
      {/each}
    {/if}

    <!-- エラーメッセージ -->
    {#if $errorMessage}
      <div class="error-banner">
        ⚠ {$errorMessage}
      </div>
    {/if}

    <!-- スクロール anchor -->
    <div bind:this={messagesEndEl}></div>
  </div>

  <!-- 入力エリア -->
  <div class="input-area">
    <div class="input-wrap">
      <textarea
        bind:this={textareaEl}
        bind:value={inputText}
        on:keydown={handleKeydown}
        on:input={adjustTextareaHeight}
        placeholder={$isLoading ? '応答を待っています...' : 'メッセージを入力（Enter で送信、Shift+Enter で改行）'}
        disabled={$isLoading}
        rows="1"
      ></textarea>
      <button
        class="send-btn"
        on:click={handleSubmit}
        disabled={$isLoading || !inputText.trim()}
        title="送信"
      >
        {#if $isLoading}
          <span class="spinner">⟳</span>
        {:else}
          ▶
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .chat-window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0f0f1a;
    color: #e0e0f0;
    flex: 1;
    min-width: 0;
  }

  /* ツールバー */
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: #1a1a2e;
    border-bottom: 1px solid #2d2d4e;
    height: 48px;
    flex-shrink: 0;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .session-badge {
    font-size: 12px;
    color: #6b6b8d;
    background: #2d2d4e;
    padding: 3px 8px;
    border-radius: 4px;
  }

  .session-badge.dimmed {
    opacity: 0.5;
  }

  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .rag-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 13px;
    color: #c8c8e0;
    user-select: none;
  }

  .rag-toggle input[type='checkbox'] {
    width: 16px;
    height: 16px;
    cursor: pointer;
    accent-color: #4a90d9;
  }

  /* メッセージエリア */
  .messages-area {
    flex: 1;
    overflow-y: auto;
    padding: 16px 0;
    scroll-behavior: smooth;
  }

  .messages-area::-webkit-scrollbar {
    width: 4px;
  }

  .messages-area::-webkit-scrollbar-thumb {
    background: #3d3d5c;
    border-radius: 2px;
  }

  /* ウェルカム画面 */
  .welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 400px;
    text-align: center;
    color: #6b6b8d;
    padding: 40px;
  }

  .welcome-icon {
    font-size: 48px;
    margin-bottom: 16px;
  }

  .welcome h2 {
    font-size: 24px;
    font-weight: 600;
    color: #c8c8e0;
    margin: 0 0 8px;
  }

  .welcome p {
    font-size: 14px;
    margin: 0 0 24px;
  }

  .hints {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 480px;
  }

  .hints li {
    font-size: 13px;
    background: #1a1a2e;
    padding: 8px 14px;
    border-radius: 8px;
    border: 1px solid #2d2d4e;
    text-align: left;
  }

  /* エラーバナー */
  .error-banner {
    margin: 8px 16px;
    padding: 10px 14px;
    background: rgba(192, 57, 43, 0.2);
    border: 1px solid #c0392b;
    border-radius: 8px;
    color: #e74c3c;
    font-size: 13px;
  }

  /* 入力エリア */
  .input-area {
    padding: 12px 16px;
    background: #1a1a2e;
    border-top: 1px solid #2d2d4e;
    flex-shrink: 0;
  }

  .input-wrap {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    background: #2d2d4e;
    border: 1px solid #3d3d5c;
    border-radius: 12px;
    padding: 8px 12px;
    transition: border-color 0.2s;
  }

  .input-wrap:focus-within {
    border-color: #4a90d9;
  }

  textarea {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #e0e0f0;
    font-size: 14px;
    line-height: 1.5;
    resize: none;
    min-height: 24px;
    max-height: 160px;
    font-family: inherit;
    overflow-y: auto;
  }

  textarea::placeholder {
    color: #6b6b8d;
  }

  textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .send-btn {
    flex-shrink: 0;
    width: 34px;
    height: 34px;
    background: #4a90d9;
    color: #fff;
    border: none;
    border-radius: 8px;
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;
  }

  .send-btn:hover:not(:disabled) {
    background: #357abd;
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
