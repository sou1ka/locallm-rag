<!-- lib/components/IngestPanel.svelte -->
<!-- File ingestion panel for adding documents to the RAG index -->

<script>
  import { open } from '@tauri-apps/api/dialog';
  import {
    isIngesting,
    indexStats,
    ingestPath,
    resetIndex,
    refreshIndexStats,
  } from '../stores/chat.js';
  import { onMount } from 'svelte';

  let ingestPathValue = '';
  let extensions = '';
  let ingestResult = null;
  let showResetConfirm = false;

  onMount(async () => {
    await refreshIndexStats();
  });

  // フォルダ選択ダイアログ
  async function handleBrowse() {
    try {
      const selected = await open({
        directory: true,   // フォルダ選択
        multiple: false,
        title: 'インデックス対象フォルダを選択',
      });
      if (selected) {
        ingestPathValue = selected;
      }
    } catch (e) {
      console.error('Failed to open dialog:', e);
    }
  }

  // ファイル選択ダイアログ
  async function handleBrowseFile() {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: 'インデックス対象ファイルを選択',
        filters: [
          { name: 'Documents', extensions: ['pdf', 'md', 'txt', 'html', 'csv', 'xlsx'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });
      if (selected) {
        ingestPathValue = selected;
      }
    } catch (e) {
      console.error('Failed to open dialog:', e);
    }
  }

  async function handleIngest() {
    if (!ingestPathValue.trim() || $isIngesting) return;
    ingestResult = null;
    const exts = extensions.trim()
      ? extensions.split(',').map((e) => e.trim().replace(/^\./, ''))
      : null;
    try {
      const result = await ingestPath(ingestPathValue.trim(), exts);
      ingestResult = {
        success: true,
        message: `完了: ${result.files_processed} ファイル / ${result.total_chunks} チャンク`,
      };
    } catch (e) {
      ingestResult = { success: false, message: `エラー: ${e}` };
    }
  }

  async function handleReset() {
    if (!showResetConfirm) {
      showResetConfirm = true;
      return;
    }
    try {
      await resetIndex();
      ingestResult = { success: true, message: 'インデックスをリセットしました' };
    } catch (e) {
      ingestResult = { success: false, message: `リセット失敗: ${e}` };
    }
    showResetConfirm = false;
  }

  function cancelReset() {
    showResetConfirm = false;
  }
</script>

<div class="ingest-panel">
  <h3 class="panel-title"><img src="/img/folder.svg" alt="📂" /> インデックス</h3>

  <!-- インデックス統計 -->
  <div class="stats-section">
    <div class="stat-row">
      <span class="stat-label">総チャンク数</span>
      <span class="stat-value">{$indexStats.total_chunks}</span>
    </div>

    {#if $indexStats.by_source_type.length > 0}
      <div class="source-breakdown">
        {#each $indexStats.by_source_type as [type, count]}
          <div class="source-row">
            <span class="source-type">{type}</span>
            <div class="source-bar-wrap">
              <div
                class="source-bar"
                style="width: {Math.round((count / $indexStats.total_chunks) * 100)}%"
              ></div>
            </div>
            <span class="source-count">{count}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="divider"></div>

  <!-- インジェストフォーム -->
  <div class="form-section">
    <label class="form-label" for="ingest-path">
    パス（ファイルまたはディレクトリ）
    </label>
    <div class="path-input-wrap">
    <input
        id="ingest-path"
        type="text"
        class="form-input path-input"
        bind:value={ingestPathValue}
        placeholder="パスを入力またはボタンで選択"
        disabled={$isIngesting}
        on:keydown={(e) => e.key === 'Enter' && handleIngest()}
    />
    <div class="browse-buttons">
        <button
        class="browse-btn"
        on:click={handleBrowseFile}
        disabled={$isIngesting}
        title="ファイルを選択"
        >
        <img src="/img/file.svg" alt="📄" />
        </button>
        <button
        class="browse-btn"
        on:click={handleBrowse}
        disabled={$isIngesting}
        title="フォルダを選択"
        >
        <img src="/img/folder.svg" alt="📂" />
        </button>
    </div>
    </div>

    <label class="form-label" for="extensions">
    拡張子フィルタ（省略で全種別）
    </label>
    <input
    id="extensions"
    type="text"
    class="form-input"
    bind:value={extensions}
    placeholder="例: md, txt, pdf"
    disabled={$isIngesting}
    />

    <button
    class="ingest-btn"
    on:click={handleIngest}
    disabled={$isIngesting || !ingestPathValue.trim()}
    >
    {#if $isIngesting}
        <span class="spinner">⟳</span>
        処理中...
    {:else}
        <img src="/img/home.svg" alt="▶" /> インデックス実行
    {/if}
    </button>
  </div>

  <!-- 結果表示 -->
  {#if ingestResult}
    <div
      class="result-banner"
      class:success={ingestResult.success}
      class:error={!ingestResult.success}
    >
      {ingestResult.success ? '✓' : '✗'}
      {ingestResult.message}
    </div>
  {/if}

  <div class="divider"></div>

  <!-- リセット -->
  <div class="reset-section">
    {#if showResetConfirm}
      <p class="confirm-text">⚠ インデックスを全削除します。よろしいですか？</p>
      <div class="confirm-buttons">
        <button class="confirm-btn danger" on:click={handleReset}>
          削除する
        </button>
        <button class="confirm-btn cancel" on:click={cancelReset}>
          キャンセル
        </button>
      </div>
    {:else}
      <button
        class="reset-btn"
        on:click={handleReset}
        disabled={$isIngesting || $indexStats.total_chunks === 0}
      >
        <img src="/img/break.svg" alt="🗑" /> インデックスをリセット
      </button>
    {/if}
  </div>
</div>

<style>
  .ingest-panel {
    padding: 16px;
    font-size: 13px;
  }

  .panel-title {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 12px;
  }

  /* 統計 */
  .stats-section {
    background: #ddd;
    border-radius: 8px;
    padding: 10px 12px;
    margin-bottom: 12px;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .stat-label {
    font-size: 12px;
  }

  .stat-value {
    font-size: 18px;
    font-weight: 700;
  }

  .source-breakdown {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .source-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .source-type {
    width: 72px;
    font-size: 11px;
    text-align: right;
    flex-shrink: 0;
  }

  .source-bar-wrap {
    flex: 1;
    height: 6px;
    background: #999;
    border-radius: 3px;
    overflow: hidden;
  }

  .source-bar {
    height: 100%;
    background: #555;
    border-radius: 3px;
    min-width: 4px;
    transition: width 0.3s;
  }

  .source-count {
    width: 30px;
    font-size: 11px;
    text-align: right;
    flex-shrink: 0;
  }

  .divider {
    height: 1px;
    background: var(--text-color);
    margin: 12px 0;
  }

  /* フォーム */
  .form-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-label {
    font-size: 11px;
  }

  .form-input {
    width: 100%;
    padding: 7px 10px;
    border: 1px solid var(--text-color);
    border-radius: 6px;
    font-size: 12px;
    outline: none;
    box-sizing: border-box;
    transition: border-color 0.2s;
  }

  .form-input:focus {

  }

  .form-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .form-input::placeholder {
    color: #999;
  }

  .ingest-btn {
    margin-top: 4px;
    padding: 8px 14px;
    background: #aaa;
    color: var(--text-color);
    border: none;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    transition: background 0.2s;
  }

  .ingest-btn:hover:not(:disabled) {
    background: #ccc;
  }

  .ingest-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* 結果バナー */
  .result-banner {
    margin-top: 10px;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 12px;
  }

  .result-banner.success {
    background: rgba(39, 174, 96, 0.2);
    border: 1px solid #27ae60;
    color: #2ecc71;
  }

  .result-banner.error {
    background: rgba(192, 57, 43, 0.2);
    border: 1px solid #c0392b;
    color: #e74c3c;
  }

  /* リセット */
  .reset-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .reset-btn {
    width: 100%;
    padding: 7px 12px;
    background: transparent;
    color: #c0392b;
    border: 1px solid #c0392b;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .reset-btn:hover:not(:disabled) {
    background: rgba(192, 57, 43, 0.15);
  }

  .reset-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .confirm-text {
    font-size: 12px;
    color: #e74c3c;
    margin: 0;
  }

  .confirm-buttons {
    display: flex;
    gap: 8px;
  }

  .confirm-btn {
    flex: 1;
    padding: 7px;
    border: none;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .confirm-btn.danger {
    background: #c0392b;
    color: #fff;
  }

  .confirm-btn.cancel {
    background: #212926;
    color: #b8b8b8;
  }

  .confirm-btn:hover {
    opacity: 0.85;
  }

  .spinner {
    display: inline-block;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

    .path-input-wrap {
        display: flex;
        gap: 4px;
        align-items: center;
    }

    .path-input {
        flex: 1;
        min-width: 0;
    }

    .browse-buttons {
        display: flex;
        gap: 2px;
        flex-shrink: 0;
    }

    .browse-btn {
        width: 30px;
        height: 30px;
        background: var(--bg-color);
        border: 1px solid var(--text-color);
        border-radius: 6px;
        color: var(--bg-color);
        filter: invert(100%);
        font-size: 14px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: background 0.15s;
        padding: 0;
    }

    .browse-btn:hover:not(:disabled) {
        background: #999;
    }

    .browse-btn:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }
</style>
