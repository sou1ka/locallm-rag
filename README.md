# LOCALLM_RAG

A local RAG (Retrieval-Augmented Generation) engine for Ollama, built entirely in Rust with Pure Rust dependencies.

## Overview

**LOCALLM_RAG** is a Rust-based RAG engine designed for local LLM environments (Ollama). It features:

- **Pure Rust HNSW Vector Index** (hnswlib-rs) - No C dependencies, fully compatible with Windows/macOS/Linux
- **Japanese-Optimized Embeddings** (ruri-v3-310m via fastembed-rs)
- **Multi-Format Document Ingestion** (PDF, text, images, audio, video, spreadsheets, SQLite, etc.)
- **Intelligent OCR** (Tesseract via leptess) for scanned PDFs and images
- **Conversation Management** with automatic history compression and JSON persistence
- **Monorepo Architecture** - CLI and Tauri desktop app sharing a common engine (rag-core)

## Project Structure

```
locallm-rag/
├── crates/
│   ├── rag-core/              # RAG engine library (core logic)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs       # Configuration parsing
│   │       ├── ingestor/       # Document ingestion (PDF, text, images, audio, video, etc.)
│   │       ├── chunker.rs      # Text chunking with lindera (Japanese NLP)
│   │       ├── embedder.rs     # fastembed wrapper
│   │       ├── store.rs        # hnswlib-rs index + chunks.json persistence
│   │       ├── retriever.rs    # Vector similarity search
│   │       ├── llm.rs          # Ollama integration (async-openai)
│   │       ├── conversation.rs # History management & summarization
│   │       └── history.rs      # JSON-based conversation persistence
│   ├── rag-cli/               # Command-line interface (uses rag-core directly)
│   │   └── src/
│   │       ├── main.rs
│   │       └── commands/
│   │           ├── chat.rs
│   │           ├── ingest.rs
│   │           ├── query.rs
│   │           └── index.rs
│   └── rag-ui/                # Desktop application (uses rag-core directly)
│       ├── src-tauri/         # Tauri backend (Rust)
│       │   └── src/
│       │       ├── main.rs
│       │       ├── commands.rs # Tauri commands
│       │       └── state.rs    # AppState
│       └── src/               # Svelte frontend
│           ├── routes/
│           └── lib/
│               ├── components/ # Sidebar, ChatWindow, IngestPanel, MessageBubble
│               ├── stores/     # Svelte stores (chat.js)
│               └── tauri.js    # Tauri API wrappers
├── config.toml                # Configuration file
├── data/
│   ├── index.bin              # HNSW index (binary)
│   ├── chunks.json            # Chunk metadata & content
│   └── summaries/             # Markdown summaries of compressed conversations
└── history/                   # Conversation history (JSON, one file per session)
    └── {YYYYMMDD_HHMMSS}_{title}.json
```

## Technology Stack

| Component | Technology | Notes |
|-----------|-----------|-------|
| Language | Rust | All backend |
| Vector Index | hnswlib-rs | Pure Rust, no C deps, Windows-compatible |
| Serialization | bincode | Binary serialization for HNSW index |
| Embeddings | fastembed-rs | ONNX runtime, Pure Rust |
| Embedding Model | ruri-v3-310m (ONNX) | Japanese-optimized, JMTEB top-tier |
| OCR | leptess | Tesseract bindings, Japanese tessdata support |
| LLM Client | async-openai | Ollama via OpenAI-compatible API |
| Conversation History | JSON files | One file per session under `history/` |
| File Formats | Multiple | PDF, CSV, Excel, text, markdown, HTML, SQLite, media files |
| File Watching | notify | Auto-ingest on file changes |
| NLP | lindera | Japanese morphological analysis for chunking |
| Frontend | Tauri + Svelte | Cross-platform desktop app |

## Quick Start

### Prerequisites

- **Rust 1.70+** and Cargo
- **Node.js** and yarn (for rag-ui frontend)
- **Ollama** running locally (`http://localhost:11434`)
- Optional: Tesseract-OCR installed for PDF/image OCR support
  ```bash
  # Ubuntu/Debian
  sudo apt-get install tesseract-ocr tesseract-ocr-jpn

  # macOS
  brew install tesseract tesseract-lang
  ```

### Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/yourusername/locallm-rag.git
   cd locallm-rag
   ```

2. **Download embedding model** (first time only)
   ```bash
   mkdir -p models
   huggingface-cli download keitokei1994/ruri-v3-310m-onnx \
       --local-dir ./models/ruri-v3-310m
   ```

3. **Configure** (edit `config.toml`)
   ```toml
   [rag]
   index_path = "./data/index.bin"
   chunks_path = "./data/chunks.json"
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

   [llm]
   base_url = "http://localhost:11434/v1"
   model = "llama3.2"
   api_key = "ollama"

   [[sources]]
   type = "directory"
   path = "./docs"
   extensions = ["md", "txt", "html"]
   ```

4. **Build**
   ```bash
   cargo build --release
   ```

### Running

#### CLI
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

#### Desktop App (rag-ui)
```bash
cd crates/rag-ui
yarn install
yarn tauri dev
```

## Core Features

### Document Ingestion

Supports multiple file types:

| Format | Handler | Details |
|--------|---------|---------|
| `.txt`, `.md`, `.html` | Text parser | Direct text extraction |
| `.pdf` | pdf-extract + leptess | Auto-detects text vs. scanned PDFs |
| `.jpg`, `.png`, `.tiff` | leptess (OCR) | Tesseract with Japanese support |
| `.csv`, `.xlsx` | calamine, csv | Row-by-row vectorization |
| `.sqlite` | rusqlite | Table-by-table vectorization |
| `.sql` | SQL parser | INSERT statement parsing |
| `.mp3`, `.flac`, `.m4a` | id3, metaflac, mp4ameta | ID3/metadata tag extraction |
| `.mp4`, `.mkv` | ffprobe + .srt | Metadata + subtitle ingestion |

### Vector Indexing

- **Storage**: HNSW graph (binary `data/index.bin`) + metadata (`data/chunks.json`)
- **In-Memory**: Index and model loaded at startup
- **No Server**: Embedded vector index, no external database needed
- **Retrieval**: O(1) lookup from chunk ID

### Conversation Management

- **JSON History**: Each session saved as `{YYYYMMDD_HHMMSS}_{title}.json` under `history/`
- **Auto-Summarization**: Compresses old messages to reduce context window usage
- **Session Support**: Multiple independent conversations, resumable by session ID
- **Markdown Exports**: Summaries saved as `.md` files under `data/summaries/`

### Streaming Support

- Real-time token streaming from Ollama
- Event-based UI updates in rag-ui (Tauri `stream_token` event)

## Implementation Status

- [x] config.rs - Configuration parsing
- [x] llm.rs - Ollama integration (async-openai)
- [x] embedder.rs - Embeddings with fastembed
- [x] store.rs - hnswlib-rs index persistence
- [x] chunker.rs - Text chunking with lindera
- [x] ingestor/ - Document ingestion
- [x] retriever.rs - Vector search
- [x] conversation.rs - History management & summarization
- [x] history.rs - JSON conversation persistence
- [x] rag-cli - CLI commands (ingest / query / chat / index)
- [x] rag-ui - Desktop UI (Tauri + Svelte)
- [ ] rag-server - Optional OpenAI-compatible HTTP server for LAN sharing

## Architecture

### Process Model

CLI and rag-ui both use rag-core directly (no daemon):

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

1. User submits query
2. Query is vectorized (fastembed + ruri-v3 `"クエリ: "` prefix)
3. Top-K similar chunks retrieved from HNSW
4. Context + history + query combined into prompt
5. Prompt sent to Ollama via OpenAI-compatible API
6. Response streamed back to UI

## Configuration Reference

See `config.toml` for full options. Key sections:

- `[rag]` - Index paths, chunk sizes, retrieval parameters
- `[embedder]` - Model files, prefixes
- `[ocr]` - Tesseract settings
- `[llm]` - Ollama connection
- `[conversation]` - History limits, summarization, history/summary directories
- `[[sources]]` - Document source definitions

## Limitations & Known Issues

- Windows: Tesseract-OCR must be installed separately
- Large documents: May exceed context window after chunking
- Concurrent access: Simultaneous ingest from CLI and rag-ui may cause index lock contention

## License

MIT
