<!-- lib/components/MessageBubble.svelte -->
<!-- Single message bubble for chat display -->

<script>
  import { marked } from 'marked';

  export let role = 'user';
  export let content = '';
  export let createdAt = '';
  export let streaming = false;

  // marked の設定
  marked.setOptions({
    breaks: true,    // 改行を<br>に変換
    gfm: true,       // GitHub Flavored Markdown
  });

  function renderContent(text) {
    if (!text) return '';
    try {
      return marked.parse(text);
    } catch {
      return text;
    }
  }

  function formatTime(dateStr) {
    if (!dateStr) return '';
    try {
      const num = Number(dateStr.replace(/\D/g, ''));
      const date = isNaN(num) || num === 0 ? new Date(dateStr) : new Date(num * 1000);
      return date.toLocaleTimeString('ja-JP', {
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return '';
    }
  }

  $: isUser = role === 'user';
  $: isAssistant = role === 'assistant';
  $: renderedContent = renderContent(content);
  $: timeStr = formatTime(createdAt);
</script>

<div class="message-wrap" class:user={isUser} class:assistant={isAssistant}>
  <!-- アバター -->
  <div class="avatar" class:user-avatar={isUser} class:assistant-avatar={isAssistant}>
    {#if isUser}
      👤
    {:else}
      <img src="/img/icon.png" alt="🤖" />
    {/if}
  </div>

  <!-- バブル本体 -->
  <div class="bubble" class:user-bubble={isUser} class:assistant-bubble={isAssistant}>
    <!-- メッセージ本文 -->
    <div class="content">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      {@html renderedContent}
      {#if streaming}
        <span class="cursor">▊</span>
      {/if}
    </div>

    <!-- タイムスタンプ -->
    {#if timeStr}
      <div class="timestamp">{timeStr}</div>
    {/if}
  </div>
</div>

<style>
  img[src="/img/icon.png"] {
    width: 24px;
  }

  .message-wrap {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 16px;
    max-width: 100%;
  }

  .message-wrap.user {
    flex-direction: row-reverse;
  }

  .message-wrap.assistant {
    flex-direction: row;
  }

  .avatar {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    margin-top: 2px;
  }

  .user-avatar {
    background: #cdc;
  }

  .assistant-avatar {
    background: #383333;
  }

  .bubble {
    max-width: 72%;
    padding: 10px 14px;
    border-radius: 12px;
    font-size: 14px;
    line-height: 1.6;
    word-break: break-word;
  }

  .user-bubble {
    background: #ccc;
    color: #111;
    border-bottom-right-radius: 4px;
  }

  .assistant-bubble {
    background: #383333;
    color: #f0e0e0;
    border-bottom-left-radius: 4px;
  }

  .content {
    white-space: pre-wrap;
  }

  /* コードブロック */
  .content :global(pre) {
    background: #0d0d0d;
    border: 1px solid #3d3d3c;
    border-radius: 6px;
    padding: 10px 12px;
    overflow-x: auto;
    margin: 8px 0;
    font-size: 13px;
  }

  .content :global(code.inline) {
    background: rgba(0, 0, 0, 0.3);
    border-radius: 3px;
    padding: 1px 5px;
    font-size: 13px;
    font-family: 'Consolas', 'Courier New', monospace;
  }

  .content :global(pre code) {
    background: transparent;
    padding: 0;
    font-family: 'Consolas', 'Courier New', monospace;
  }

  /* ストリーミングカーソル */
  .cursor {
    display: inline-block;
    animation: blink 0.8s step-end infinite;
    color: #4a90d9;
    font-weight: bold;
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .timestamp {
    font-size: 11px;
    opacity: 0.5;
    margin-top: 4px;
    text-align: right;
  }

  .message-wrap.user .timestamp {
    text-align: right;
  }

  .message-wrap.assistant .timestamp {
    text-align: left;
  }
</style>
