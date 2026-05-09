<!-- routes/+page.svelte -->
<script>
  import Sidebar from '$lib/components/Sidebar.svelte';
  import ChatWindow from '$lib/components/ChatWindow.svelte';
  import IngestPanel from '$lib/components/IngestPanel.svelte';
  import { appWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/api/shell';
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

  function applyTheme(theme) {
    if (theme === 'dark') {
      document.body.parentElement.classList.add('dark');
    } else {
      document.body.parentElement.classList.remove('dark');
    }
  }

  onMount(async () => {
    const theme = await appWindow.theme();
    applyTheme(theme);

    await appWindow.onThemeChanged(({ payload }) => {
      applyTheme(payload);
    });

    await refreshIndexStats();

    // リンククリック時にデフォルトブラウザで開く
    document.addEventListener('click', async (e) => {
      const target = e.target instanceof HTMLElement ? e.target.closest('a') : null;
      if (target instanceof HTMLAnchorElement) {
        const href = target.getAttribute('href');
        if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
          e.preventDefault();
          e.stopPropagation();
          try {
            await open(href);
          } catch (err) {
            console.error('Failed to open link:', err);
          }
        }
      }
    }, true);

    document.addEventListener('contextmenu', function(e) {
      e.preventDefault();
      e.stopPropagation();
    }, false);

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
          <img src="/src/img/close.svg" alt="◀" />
        </button>
        <button class="icon-btn" on:click={handleNewSession} title="新規会話">
          <img src="/src/img/comment.svg" alt="✏" />
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
          <img src="/src/img/folder.svg" alt="📂" />
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
        <img src="/src/img/menu.svg" alt="▶" />
      </button>
      <button class="icon-btn" on:click={handleNewSession} title="新規会話">
        <img src="/src/img/comment.svg" alt="✏" />
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
        <img src="/src/img/folder.svg" alt="📂" />
      </button>
    </div>
  {/if}

  <!-- メインエリア -->
  <main class="main-area">
    <ChatWindow />
  </main>
</div>

<style>
  :root {
    --text-color: #333;
    --bg-color: #eff0ef;
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    background: var(--bg-color);
    color: var(--text-color);
    font-family:
      'M PLUS 1 Code',
      'Noto Sans JP',
      'Meiryo',
      system-ui,
      sans-serif;
    overflow: hidden;
  }

  :global(img[src$=".svg"]) {
    width: 18px;
    height: 18px;
    vertical-align: top;
    filter: brightness(0) saturate(100%) invert(20%) sepia(11%) saturate(16%) hue-rotate(4deg) brightness(94%) contrast(91%);
  }

  :global(ul),:global(ol) {
    padding-inline-start: 20px;
  }

  :global(a:link), :global(a:visited) {
    color: #29f;
  }

  :global(html.dark) {
    filter: invert(1) hue-rotate(180deg);
  }

  :global(html.dark img[src$=".png"], html.dark input[type="checkbox"]) {
      filter: invert(1) hue-rotate(180deg);
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
    border-right: 1px solid var(--text-color);
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
    background: var(--text-color);
    margin: 0 10px 4px;
    flex-shrink: 0;
  }

  /* アイコンボタン共通 */
  .icon-btn {
    width: 32px;
    height: 32px;
    background: transparent;
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
    background: #aaa;
  }

  .icon-btn.active {
    background: #aaa;
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
    background: #aaa;
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
