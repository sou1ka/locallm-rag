# LOCALLM_RAG

A local RAG (Retrieval-Augmented Generation) system for local LLMs, built entirely in Rust.

ローカルLLM向けのRAG（検索拡張生成）システムです。バックエンドはすべてRustで実装されています。

---

## Overview

**LOCALLM_RAG** is a Rust-based RAG engine designed for local LLM environments. It features:

- **Japanese-Optimized Embeddings** — ruri-v3-310m (ONNX) via fastembed-rs
- **Multi-Format Document Ingestion** — PDF, text, Markdown, HTML, CSV, Excel, SQLite, media files, and more
- **SQLite Vector Store** — Embeddings and chunk metadata stored in a single `.db` file
- **Conversation Management** — Session history with automatic summarization
- **Monorepo Architecture** — CLI, desktop app, and HTTP server sharing a common engine (rag-core)

**LOCALLM_RAG**はローカルLLM環境向けのRAGエンジンです。

- 日本語に最適化された埋め込みモデル（ruri-v3-310m / ONNX）を使用
- PDF・テキスト・CSV・Excel・SQLiteなど多様なファイル形式に対応
- ベクトルデータをSQLiteに保存（単一の `.db` ファイルで管理）
- 会話履歴の保持と自動要約圧縮
- CLI・デスクトップアプリ・HTTPサーバーが共通のエンジンを使用するモノレポ構成

## Project Structure

```
locallm-rag/
├── crates/
│   ├── rag-core/              # RAG engine library (core logic)
│   │   └── src/
│   │       ├── config.rs
│   │       ├── lib.rs
│   │       ├── chunker.rs      # Text chunking with lindera (Japanese NLP)
│   │       ├── embedder.rs     # fastembed wrapper
│   │       ├── store.rs        # hnswlib-rs index + store.db persistence
│   │       ├── retriever.rs    # Vector similarity search
│   │       ├── llm.rs          # Integration of OpenAI-compatible APIs (async-openai)
│   │       ├── conversation.rs # History management & summarization
│   │       ├── history.rs      # JSON-based conversation persistence
│   │       └── ingestor/       # Document ingestion (PDF, text, images, audio, video, etc.)
│   ├── rag-cli/               # Command-line interface (uses rag-core directly)
│   │   └── src/
│   │       ├── main.rs
│   │       ├── config.rs
│   │       └── commands/
│   │           ├── chat.rs
│   │           ├── index.rs
│   │           ├── ingest.rs
│   │           ├── init.rs
│   │           ├── mod.rs
│   │           └── query.rs
│   ├── rag-ui/                # Desktop application (uses rag-core directly)
│   │   ├── src-tauri/         # Tauri backend (Rust)
│   │   │   └── src/
│   │   │       ├── main.rs
│   │   │       ├── commands.rs # Tauri commands
│   │   │       └── state.rs    # AppState
│   │   └── src/               # Svelte frontend
│   │       ├── routes/
│   │       └── lib/
│   │           ├── components/ # Sidebar, ChatWindow, IngestPanel, MessageBubble
│   │           ├── stores/     # Svelte stores (chat.js)
│   │           └── tauri.js    # Tauri API wrappers
│   └── rag-server/            # OpenAI-compatible HTTP server (uses rag-core directly)
│       └── src/
│           ├── main.rs         # Server entry point & startup
│           ├── config.rs       # Config loader (adds [server] section)
│           ├── state.rs        # AppState (Store + Embedder + LlmClient)
│           └── routes.rs       # axum routes (chat/completions, ingest, index)
├── config.toml                # Configuration file
├── data/
│   └── store.db           # SQLite: chunk metadata + embeddings
├── history/               # Conversation history (JSON, one file per session)
│   └── {YYYYMMDD_HHMMSS}_{title}.json
```

## Technology Stack

LOCALLM_RAG is built on a carefully selected stack of Rust libraries optimized for local LLM use. All components are either pure Rust or have minimal external dependencies, ensuring compatibility and portability across Windows, macOS, and Linux.

LOCALLM_RAGは、ローカルLLM環境に最適化されたRustライブラリで構成されています。すべてのコンポーネントはピュアRustまたは最小限の外部依存性を持ち、Windows、macOS、Linuxでの互換性と可搬性を確保しています。

| Component | Technology | Notes |
|-----------|-----------|-------|
| Language | Rust | All backend |
| Vector Store | SQLite | Chunk metadata + embeddings in single `.db` file |
| Vector Index | hnswlib-rs | In-memory HNSW graph for fast similarity search |
| Embeddings | fastembed-rs | ONNX runtime, Pure Rust, batch processing support |
| Embedding Model | ruri-v3-310m (ONNX) | Japanese-optimized, JMTEB top-tier |
| OCR | leptess | Tesseract bindings, Japanese tessdata support |
| LLM Client | async-openai | Ollama via OpenAI-compatible API |
| Conversation History | JSON files | One file per session under `history/` |
| File Formats | Multiple parsers | PDF, CSV, Excel, text, markdown, HTML, SQLite, media files |
| File Watching | notify | Auto-ingest on file changes |
| NLP | lindera | Japanese morphological analysis for text chunking |
| Frontend | Tauri + Svelte | Cross-platform desktop app with native performance |

## Quick Start

Get started with LOCALLM_RAG in minutes. This section covers prerequisites, installation, and configuration.

数分でLOCALLM_RAGをはじめることができます。前提条件、インストール、設定についてカバーしています。

### Prerequisites

You'll need the following tools installed:

以下のツールをインストールしている必要があります：

- **Rust 1.70+** and Cargo — Download from [rustup.rs](https://rustup.rs/)
- **Node.js** and yarn — Required only for `rag-ui` development (yarn is installed via `npm install -g yarn`)
- **Ollama** — Running locally on `http://localhost:11434`
- **Optional**: Tesseract-OCR — For PDF and image text extraction (Japanese language support)
  ```bash
  # Ubuntu/Debian
  sudo apt-get install tesseract-ocr tesseract-ocr-jpn

  # macOS
  brew install tesseract tesseract-lang

  # Windows: Download installer from https://github.com/UB-Mannheim/tesseract/wiki
  ```

### Setup

1. **Clone the repository**

   ```bash
   git clone https://github.com/yourusername/locallm-rag.git
   cd locallm-rag
   ```

2. **Download the embedding model** (first time only)

   ```bash
   mkdir -p models
   huggingface-cli download keitokei1994/ruri-v3-310m-onnx \
       --local-dir ./models/ruri-v3-310m
   ```

3. **Configure `config.toml`**

   `config.toml` を編集して設定します

   ```toml
   [rag]
   db_path = "./data/store.db"
   chunk_size = 500
   chunk_overlap = 50
   top_k = 5
   score_threshold = 0.75

   [embedder]
   model_type = "custom_onnx"
   onnx_path = "./models/ruri-v3-310m/model.onnx"
   tokenizer_path = "./models/ruri-v3-310m/tokenizer.json"
   doc_prefix = "文章: "
   query_prefix = "クエリ: "
   embed_batch_size = 128

   [llm]
   base_url = "http://localhost:11434/v1"
   model = "llama3.2"
   api_key = "ollama"

   [[sources]]
   type = "directory"
   path = "./docs"
   extensions = ["md", "txt", "html"]
   ```

4. **Build the project**

   ```bash
   cargo build --release
   ```

## Running

### Development

#### CLI

Command-line interface for ingestion and chat. Build with `cargo build --release` first.

CLIを使用してインジェストとチャットを実行します。事前に `cargo build --release` でビルドしてください。

```bash
# Ingest documents from config.toml sources
cargo run --release --bin rag-cli -- ingest

# Query with RAG
cargo run --release --bin rag-cli -- query "What is the setup process?"

# Interactive chat
cargo run --release --bin rag-cli -- chat

# Resume a previous session
cargo run --release --bin rag-cli -- chat --session 20260506_214349

# View index statistics
cargo run --release --bin rag-cli -- index stats
```

#### HTTP Server

OpenAI-compatible HTTP server for LAN use. Default binding is `127.0.0.1:18080` (configure via `config.toml [server]` section).

LANで使用するOpenAI互換HTTPサーバーです。デフォルトバインディングは `127.0.0.1:18080` です（`config.toml [server]` セクションで設定可能）。

```bash
# Start the HTTP server
cargo run --release --bin rag-server

# Chat completions (OpenAI-compatible) — new session
curl http://localhost:18080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"Hello"}]}'

# Continue an existing session (session_id is returned in the response)
curl http://localhost:18080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"Tell me more"}],"session_id":"20260506_143022"}'

# Streaming responses
curl http://localhost:18080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"Hello"}],"stream":true}'

# List all sessions
curl http://localhost:18080/v1/sessions

# Get session history
curl http://localhost:18080/v1/sessions/20260506_143022

# Delete a session
curl -X DELETE http://localhost:18080/v1/sessions/20260506_143022

# Ingest a directory
curl -X POST http://localhost:18080/v1/ingest \
  -H "Content-Type: application/json" \
  -d '{"path":"./docs","extensions":["md","txt"]}'

# Get index statistics
curl http://localhost:18080/v1/index/stats

# Reset the index
curl -X DELETE http://localhost:18080/v1/index
```

#### Desktop App

Native desktop application with graphical interface. Uses Tauri + Svelte.

グラフィカルインターフェイス付きのネイティブデスクトップアプリケーションです。Tauri + Svelteを使用しています。

```bash
cd crates/rag-ui
yarn install
yarn tauri dev
```

### Release Binaries

After building with `cargo build --release`, use the compiled binaries directly. Binaries are available as `rag.exe`, `rag-server.exe`, and `rag-ui.exe` on Windows.

`cargo build --release` でビルド後、コンパイルされたバイナリを直接使用できます。Windowsではバイナリは `rag.exe`、`rag-server.exe`、`rag-ui.exe` として利用可能です。

```bash
# CLI: Ingest documents
rag ingest

# CLI: Interactive chat
rag chat

# CLI: Query with RAG
rag query "Your question here"

# HTTP Server: Start the server
rag-server

# Desktop App: Launch the GUI
# On Windows: Double-click rag-ui.exe or run from command line
# Windowsの場合: rag-ui.exeをダブルクリックするか、コマンドラインから実行してください
rag-ui.exe
```

## Core Features

### Document Ingestion

LOCALLM_RAG supports a wide variety of document formats. Each format is automatically detected by file extension and processed through the appropriate handler. Automatic encoding detection (UTF-8 with ShiftJIS fallback) ensures compatibility with files from different sources and locales.

LOCALLM_RAGは多様なドキュメント形式をサポートしています。各形式はファイル拡張子で自動検出され、適切なハンドラを通じて処理されます。自動エンコード検出（ShiftJIS フォールバック付きUTF-8）により、異なるソースやロケールからのファイルとの互換性を確保します。

| Format | Handler | Details |
|--------|---------|---------|
| `.txt`, `.md`, `.html` | Text parser | Direct text extraction |
| `.pdf` | pdf-extract + leptess | Auto-detects text vs. scanned PDFs |
| `.jpg`, `.png`, `.tiff` | leptess (OCR) | Tesseract with Japanese support |
| `.csv`, `.xlsx` | calamine, csv | Row-by-row vectorization with automatic encoding detection |
| `.sqlite` | rusqlite | Table-by-table vectorization |
| `.sql` | SQL parser | INSERT statement parsing |
| `.mp3`, `.flac`, `.m4a` | id3, metaflac, mp4ameta | ID3/metadata tag extraction |
| `.mp4`, `.mkv` | ffprobe + .srt | Metadata + subtitle ingestion |

### Vector Indexing

Vector indexing is handled by HNSW (Hierarchical Navigable Small World) in-memory graph, with persistent storage in SQLite. This approach combines the speed of in-memory search with the reliability of database storage, without requiring external services.

ベクトルインデックスはメモリ内HNSW（階層型ナビゲート可能な小世界グラフ）グラフで処理され、SQLiteで永続的に保存されます。このアプローチは、メモリ内検索の速度と、外部サービスを必要としないデータベース保存の信頼性を組み合わせています。

- **Storage**: SQLite database (`data/store.db`) — Embeddings and chunk metadata in a single file
- **In-Memory Index**: HNSW graph loaded at startup for O(log n) nearest-neighbor search
- **No External Services**: Embedded vector search, no separate vector database server needed
- **Batch Embedding**: Configurable batch processing (default 128) for efficient memory usage during ingestion
- **Persistence**: Automatic checkpoints after ingest operations ensure data safety

### Conversation Management

LOCALLM_RAG maintains full conversation history with automatic summarization to manage token usage. Each conversation session is independently stored and can be resumed at any time, allowing for multi-turn dialogues without losing context.

LOCALLM_RAGは自動要約によってトークン使用量を管理しながら、完全な会話履歴を保持します。各会話セッションは独立して保存され、いつでも再開できるため、コンテキストを失わずにマルチターン対話が可能です。

- **JSON History**: Each session saved as `{YYYYMMDD_HHMMSS}_{title}.json` under `history/` directory
- **Auto-Summarization**: Old messages are compressed into summaries to reduce context window usage
- **Session Support**: Resume previous conversations by session ID without losing context
- **Markdown Exports**: Summary files automatically exported as `.md` files under `data/summaries/`

### Streaming Support

Real-time streaming allows responses to be displayed as they are generated, providing immediate feedback without waiting for the complete response. This is especially useful for longer responses where users can begin reading while generation continues.

リアルタイムストリーミングにより、生成中の応答をそのまま表示することで、完全な応答を待つことなく即座にフィードバックを得られます。これは特に長い応答の場合に、ユーザーが生成中に読み始めることができるため有用です。

- **Real-time Token Streaming**: Responses stream token-by-token from Ollama as they are generated
- **UI Integration**: Desktop app updates with events (`stream_token`) for smooth real-time display

## Implementation Status

- [x] config.rs - Configuration parsing
- [x] llm.rs - Ollama integration (async-openai)
- [x] embedder.rs - Embeddings with fastembed
- [x] store.rs - SQLite store + HNSW inde
- [x] chunker.rs - Text chunking with lindera
- [x] ingestor/ - Document ingestion
- [x] retriever.rs - Vector search
- [x] conversation.rs - History management & summarization
- [x] history.rs - JSON conversation persistence
- [x] rag-cli - CLI commands (ingest / query / chat / index)
- [x] rag-ui - Desktop UI (Tauri + Svelte)
- [x] rag-server - OpenAI-compatible HTTP server for LAN sharing

## Architecture

### Process Model

LOCALLM_RAG uses a direct-library architecture where both CLI and desktop app link to rag-core at compile time. This eliminates the need for a separate daemon process and reduces overhead by sharing a single loaded index and embedding model.

LOCALLM_RAGはダイレクトライブラリアーキテクチャを採用しており、CLIとデスクトップアプリの両方がコンパイル時にrag-coreにリンクされています。これにより別のデーモンプロセスが不要になり、単一の読み込みインデックスと埋め込みモデルを共有することでオーバーヘッドが削減されます。

```
┌─────────────────────┐     ┌─────────────────────┐
│  rag-ui             │     │  rag-cli            │
│  (Tauri + Svelte)   │     │  (Terminal REPL)    │
└─────────────────────┘     └─────────────────────┘
           │                           │
           └─────────────┬─────────────┘
                         │ (direct library call)
              ┌──────────▼──────────┐
              │     rag-core        │
              ├─────────────────────┤
              │ hnswlib-rs (memory) │
              │ fastembed (memory)  │
              │ Ollama (HTTP)       │
              └─────────────────────┘
```

### Query Flow

The typical flow for answering a user query is:

ユーザーのクエリに答えるための一般的な流れは以下の通りです：

1. User submits query — Query text is received from CLI, HTTP API, or desktop UI
2. Vectorization — Query is converted to embeddings using fastembed with ruri-v3 and `"クエリ: "` prefix
3. Retrieval — Top-K similar chunks retrieved from in-memory HNSW index
4. Context Building — Retrieved chunks combined with conversation history and user query into a prompt
5. LLM Inference — Complete prompt sent to Ollama via OpenAI-compatible API
6. Streaming Response — Tokens streamed back in real-time to the client (UI, CLI, or HTTP)

## Configuration Reference

All configuration is stored in `config.toml`. This file controls embedding models, LLM connection, document ingestion sources, and system behavior. See the Setup section for a complete example.

すべての設定は `config.toml` に保存されます。このファイルは埋め込みモデル、LLM接続、ドキュメント取り込みソース、システム動作を制御します。完全な例については「Setup」セクションを参照してください。

**Key Configuration Sections:**

**主な設定セクション：**

- `[rag]` — Vector store path (`db_path`), chunk sizes (`chunk_size`, `chunk_overlap`), retrieval parameters (`top_k`, `score_threshold`)
- `[embedder]` — Model files and paths, embedding prefixes (`doc_prefix`, `query_prefix`), batch size (`embed_batch_size` for performance tuning)
- `[ocr]` — Tesseract-OCR settings for PDF and image processing
- `[llm]` — Ollama connection details (`base_url`, `model`, `api_key`)
- `[conversation]` — History management (message limits, summarization thresholds), directory paths
- `[[sources]]` — Document source definitions (type, path, file extensions to ingest)

## Limitations & Known Issues

既知の制限と問題点を以下に記載します。これらはバージョン管理されており、今後のリリースで改善される予定です。

**Known Limitations:**

**既知の制限：**

- **Windows Tesseract Installation** — On Windows, Tesseract-OCR must be installed separately. Linux and macOS installations include Tesseract via package managers.
- **Large Document Context** — Very large documents may exceed LLM context windows after chunking. Consider splitting documents or adjusting `chunk_size` and `top_k` parameters.
- **Concurrent Access** — Simultaneous ingestion from multiple processes (CLI and rag-ui simultaneously) may cause index lock contention. Sequential access is recommended.
- **GPU Acceleration** — Currently uses CPU-only embedding via ONNX. GPU support is not yet implemented.

## Credits

クレジット

### Icons

UI icons are provided by [icooon-mono.com](https://icooon-mono.com/).  
The application logo (`icon.png`) is an original work by the project author.

UIアイコンは [icooon-mono.com](https://icooon-mono.com/) によって提供されています。アプリケーションロゴ（`icon.png`）はプロジェクトオーナー作です。

### AI Assistance

This project was built with the assistance of [Claude](https://claude.ai/) (Anthropic).  
The vast majority of the source code — including the Rust backend (rag-core, rag-cli, rag-server) and the Svelte frontend logic — was generated through iterative collaboration with Claude.

このプロジェクトは [Claude](https://claude.ai/) (Anthropic) の協力を得て構築されました。ソースコードのほぼすべて（Rustバックエンド（rag-core、rag-cli、rag-server）およびSvelteフロントエンドロジック）はClaudeとの反復的な協力により生成されました。

The following were created by the project author:

プロジェクト作成者により作成されたもの：

- UI color design and visual theme (Tauri / Svelte frontend)
- Application logo

## License

ライセンス

MIT — see [LICENSE](./LICENSE) for details.

MIT — 詳細は [LICENSE](./LICENSE) を参照してください。
