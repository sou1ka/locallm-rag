# LOCALLM_RAG

A local RAG (Retrieval-Augmented Generation) engine for Ollama, built entirely in Rust with Pure Rust dependencies.

## Overview

**LOCALLM_RAG** is a Rust-based RAG engine designed for local LLM environments (Ollama). It features:

- **Pure Rust HNSW Vector Index** (hnswlib-rs) - No C dependencies, fully compatible with Windows/macOS/Linux
- **Japanese-Optimized Embeddings** (ruri-v3-310m via fastembed-rs)
- **Multi-Format Document Ingestion** (PDF, text, images, audio, video, spreadsheets, SQLite, etc.)
- **Intelligent OCR** (Tesseract via leptess) for scanned PDFs and images
- **Conversation Management** with automatic history compression
- **Monorepo Architecture** - CLI, daemon, and Tauri desktop app sharing a common engine

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
│   │       └── conversation.rs # History management & summarization
│   ├── rag-cli/               # Command-line interface
│   │   └── src/main.rs
│   ├── rag-daemon/            # Background daemon (tarpc RPC server)
│   │   └── src/main.rs
│   └── rag-tauri/             # Desktop application
│       ├── src-tauri/         # Tauri backend
│       └── src/               # Svelte frontend
├── config.toml                # Configuration file
└── data/
    ├── index.bin              # HNSW index (binary)
    ├── chunks.json            # Chunk metadata & content
    ├── history.db             # SQLite conversation history
    └── summaries/             # Markdown summaries of compressed conversations
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
| Conversation DB | SQLite | rusqlite with bundled support |
| File Formats | Multiple | PDF, CSV, Excel, text, markdown, HTML, SQLite, media files |
| IPC | tarpc | Daemon ↔ CLI/Tauri communication |
| File Watching | notify | Auto-ingest on file changes |
| NLP | lindera | Japanese morphological analysis for chunking |
| Frontend | Tauri + Svelte | Cross-platform desktop app |

## Quick Start

### Prerequisites

- **Rust 1.70+** and Cargo
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

# View index statistics
cargo run --release --bin rag-cli -- index stats
```

#### Daemon + CLI
```bash
# Start daemon in background
cargo run --release --bin rag-daemon &

# Use CLI (will connect to daemon)
cargo run --release --bin rag-cli -- query "..."
```

#### Desktop App
```bash
cd crates/rag-tauri
cargo tauri dev
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

- **Storage**: HNSW graph (binary `index.bin`) + metadata (`chunks.json`)
- **In-Memory**: Index and model loaded once at daemon startup
- **No Server**: Embedded vector index, no external database needed
- **Retrieval**: O(1) lookup from chunk ID

### Conversation Management

- **SQLite History**: All messages persisted locally
- **Auto-Summarization**: Compresses old messages to reduce context window
- **Session Support**: Multiple independent conversations
- **Markdown Exports**: Summaries saved as `.md` files

### Streaming Support

- Real-time token streaming from Ollama
- Event-based UI updates in Tauri app

## Implementation Status

- [x] config.rs - Configuration parsing
- [x] llm.rs - Ollama integration (async-openai)
- [ ] embedder.rs - Embeddings with fastembed
- [ ] store.rs - hnswlib-rs index persistence
- [ ] chunker.rs - Text chunking with lindera
- [ ] ingestor/* - Document ingestion
- [ ] retriever.rs - Vector search
- [ ] conversation.rs - History management
- [ ] rag-daemon - RPC server
- [ ] rag-cli - CLI commands
- [ ] rag-tauri - Desktop UI

## Architecture

### Process Model

```
┌──────────────┐        ┌──────────────────────┐
│  Tauri App   │        │  CLI Tool            │
│  (Frontend)  │        │                      │
└──────────────┘        └──────────────────────┘
       │                          │
       └──────────┬───────────────┘
                  │
         Unix Domain Socket (tarpc)
                  │
       ┌──────────▼───────────┐
       │   rag-daemon         │
       ├──────────────────────┤
       │ hnswlib-rs (memory)  │
       │ fastembed (memory)   │
       │ Ollama (HTTP)        │
       └──────────────────────┘
```

### Query Flow

1. User submits query
2. Query is vectorized (fastembed + ruri-v3 prefix)
3. Top-K similar chunks retrieved from HNSW
4. Context + history + query combined into prompt
5. Prompt sent to Ollama via OpenAI API
6. Response streamed back to UI

## Configuration Reference

See `config.toml` for full options. Key sections:

- `[rag]` - Index paths, chunk sizes, retrieval parameters
- `[embedder]` - Model files, prefixes
- `[ocr]` - Tesseract settings
- `[llm]` - Ollama connection
- `[conversation]` - History limits, summarization
- `[daemon]` - Socket and PID paths
- `[[sources]]` - Document source definitions

## Limitations & Known Issues

- Windows: Tesseract-OCR must be installed separately
- Large documents: May exceed context window after chunking
- Real-time sync: Index changes require daemon restart

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Submit a pull request

## License

MIT

## Support

- Issues: GitHub Issues
- Discussions: GitHub Discussions
- Documentation: See `idea.md` for detailed specification