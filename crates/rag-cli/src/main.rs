//! rag-cli entry point
//!
//! Command-line interface for LOCALLM_RAG.
//! Provides ingest, query, chat, and index management commands.

mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rag",
    version = "0.1.0",
    about = "LOCALLM_RAG - Japanese-optimized local LLM with RAG",
    long_about = None,
)]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest files into the vector index
    Ingest {
        /// Path to file or directory to ingest
        #[arg(default_value = ".")]
        path: String,

        /// File extensions to include (e.g. --ext md --ext txt)
        #[arg(short, long = "ext")]
        extensions: Vec<String>,

        /// Watch for file changes and auto-reingest
        #[arg(short, long)]
        watch: bool,
    },

    /// Query the RAG pipeline
    Query {
        /// Query text
        text: String,

        /// Bypass RAG and send directly to LLM
        #[arg(long)]
        no_rag: bool,

        /// Number of chunks to retrieve
        #[arg(long)]
        top_k: Option<usize>,

        /// Show retrieved context chunks
        #[arg(long)]
        show_context: bool,
    },

    /// Start an interactive chat session
    Chat {
        /// Session ID to resume (e.g. 20260506_143022)
        #[arg(short, long)]
        session: Option<String>,

        /// Bypass RAG and send directly to LLM
        #[arg(long)]
        no_rag: bool,

        /// List past conversation sessions
        #[arg(short, long)]
        list: bool,
    },

    /// Manage the vector index
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },

    /// Initialize models (download ruri-v3 ONNX)
    Init,
}

#[derive(Subcommand)]
enum IndexAction {
    /// Show index statistics
    Stats,

    /// Reset the entire index
    Reset {
        /// Reset only chunks from this source path
        #[arg(long)]
        source: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // init コマンドだけ validate なしでロード
    let config = if matches!(cli.command, Commands::Init) {
        match config::load_without_validate(&cli.config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error: Failed to load config '{}': {}", cli.config, e);
                std::process::exit(1);
            }
        }
    } else {
        match config::load(&cli.config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error: Failed to load config '{}': {}", cli.config, e);
                std::process::exit(1);
            }
        }
    };

    // Dispatch to subcommand
    let result = match cli.command {
        Commands::Ingest { path, extensions, watch } => {
            let exts = if extensions.is_empty() {
                None
            } else {
                Some(extensions)
            };
            commands::ingest::run(&config, &path, exts, watch).await
        }

        Commands::Query { text, no_rag, top_k, show_context } => {
            commands::query::run(&config, &text, !no_rag, top_k, show_context).await
        }

        Commands::Chat { session, no_rag, list } => {
            if list {
                commands::chat::list_sessions(&config).await
            } else {
                commands::chat::run(&config, session.as_deref(), !no_rag).await
            }
        }

        Commands::Index { action } => match action {
            IndexAction::Stats => {
                commands::index::stats(&config).await
            }
            IndexAction::Reset { source, yes } => {
                commands::index::reset(&config, source.as_deref(), yes).await
            }
        },

        Commands::Init => {
            let config = match config::load_without_validate(&cli.config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error: Failed to load config '{}': {}", cli.config, e);
                    std::process::exit(1);
                }
            };
            commands::init::run(&config).await
        }
    };

    // Handle errors
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
