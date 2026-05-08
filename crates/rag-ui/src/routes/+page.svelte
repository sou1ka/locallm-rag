<!-- routes/+page.svelte -->
<script>
  import Sidebar from '$lib/components/Sidebar.svelte';
  import ChatWindow from '$lib/components/ChatWindow.svelte';
  import IngestPanel from '$lib/components/IngestPanel.svelte';
  import { onMount } from 'svelte';
  import { refreshIndexStats, newSession, saveAllSessions } from '$lib/stores/chat.js';

  // localStorageから初期値を復元
  let sidebarOpen = typeof localStorage !== 'undefined'
    ? localStorage.getItem('sidebarOpen') !== 'false'
    : true;

  let activeTab = typeof localStorage !== 'undefined'
    ? (localStorage.getItem('sidebarActiveTab') || 'history')
    : 'history';

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
    localStorage.setItem('sidebarOpen', String(sidebarOpen));
  }

  onMount(async () => {
    await refreshIndexStats();

    document.addEventListener('contextmenu', function(e) {
      e.preventDefault();
      e.stopPropagation();
    }, false);

    document.addEventListener('selectstart', function(e) {
      e.preventDefault();
      e.stopPropagation();
    });

    document.addEventListener('keydown', async function(e) {
      if(e.key == 'F5' || (e.ctrlKey && e.key == 'r') || e.key == 'F7') {
        e.preventDefault();
        e.stopPropagation();
      }
    });
  });

  function handleNewSession() {
    newSession();
    activeTab = 'history';
  }
</script>

<div class="app-layout">
  {#if sidebarOpen}
    <!-- サイドバー展開時 -->
    <div class="sidebar-wrap open">
      <!-- サイドバーヘッダー -->
      <div class="sidebar-header">
        <button class="icon-btn" on:click={toggleSidebar} title="サイドバーを閉じる">
          ◀
        </button>
        <button class="icon-btn" on:click={handleNewSession} title="新規会話">
          ✏
        </button>
        <button
          class="icon-btn"
          class:active={activeTab === 'ingest'}
          on:click={() => {
            activeTab = activeTab === 'ingest' ? 'history' : 'ingest';
            localStorage.setItem('sidebarActiveTab', activeTab);
          }}
          title="インデックス"
        >
          📂
        </button>
      </div>

      <div class="sidebar-divider"></div>

      <!-- タブコンテンツ -->
      {#if activeTab === 'history'}
        <Sidebar />
      {:else}
        <div class="ingest-scroll">
          <IngestPanel />
        </div>
      {/if}
    </div>
  {:else}
    <!-- サイドバー折り畳み時（アイコンのみ） -->
    <div class="sidebar-wrap closed">
      <button class="icon-btn" on:click={toggleSidebar} title="サイドバーを開く">
        ▶
      </button>
      <button class="icon-btn" on:click={handleNewSession} title="新規会話">
        ✏
      </button>
      <button
        class="icon-btn"
        on:click={() => {
          sidebarOpen = true;
          activeTab = 'ingest';
          localStorage.setItem('sidebarOpen', 'true');
          localStorage.setItem('sidebarActiveTab', 'ingest');
        }}
        title="インデックス"
      >
        📂
      </button>
    </div>
  {/if}

  <!-- メインエリア -->
  <main class="main-area">
    <ChatWindow />
  </main>
</div>

<style>
  :global(*, *::before, *::after) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    background: #0f0f1a;
    color: #e0e0f0;
    font-family:
      'Noto Sans JP',
      'Hiragino Sans',
      'Yu Gothic',
      'Meiryo',
      system-ui,
      sans-serif;
    overflow: hidden;
  }

  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  /* サイドバー共通 */
  .sidebar-wrap {
    display: flex;
    flex-direction: column;
    background: #1a1a2e;
    border-right: 1px solid #2d2d4e;
    flex-shrink: 0;
    transition: width 0.2s;
  }

  .sidebar-wrap.open {
    width: 260px;
    min-width: 200px;
  }

  .sidebar-wrap.closed {
    width: 48px;
    align-items: center;
    padding: 8px 0;
    gap: 8px;
  }

  /* サイドバーヘッダー（展開時） */
  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 10px;
    flex-shrink: 0;
  }

  .sidebar-divider {
    height: 1px;
    background: #2d2d4e;
    margin: 0 10px 4px;
    flex-shrink: 0;
  }

  /* アイコンボタン共通 */
  .icon-btn {
    width: 32px;
    height: 32px;
    background: transparent;
    color: #8888aa;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s;
    flex-shrink: 0;
  }

  .icon-btn:hover {
    background: #2d2d4e;
    color: #c8c8e0;
  }

  .icon-btn.active {
    color: #4a90d9;
    background: #2d2d4e;
  }

  /* インジェストパネルのスクロール */
  .ingest-scroll {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .ingest-scroll::-webkit-scrollbar {
    width: 4px;
  }

  .ingest-scroll::-webkit-scrollbar-thumb {
    background: #3d3d5c;
    border-radius: 2px;
  }

  /* メインエリア */
  .main-area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
