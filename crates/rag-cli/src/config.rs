//! CLI configuration loader
//!
//! Loads and validates config.toml for rag-cli.
//! Wraps rag-core config types with CLI-specific defaults.

use anyhow::{Context, Result};
use rag_core::config::{
    ConversationConfig, EmbedderConfig, LlmConfig, OcrConfig, RagConfig,
};
use serde::Deserialize;
use std::path::Path;

/// Top-level config matching config.toml
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub rag: RagConfig,
    pub embedder: EmbedderConfig,
    pub llm: LlmConfig,
    pub conversation: ConversationConfig,
    pub ocr: Option<OcrConfig>,
    pub sources: Option<Vec<SourceConfig>>,
}

/// Source definition in config.toml [[sources]]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceConfig {
    Directory {
        path: String,
        extensions: Option<Vec<String>>,
    },
    Pdf {
        path: String,
    },
    Sqlite {
        path: String,
        tables: Option<Vec<String>>,
    },
    SqliteDump {
        path: String,
    },
    Csv {
        path: String,
    },
}

/// Load config from path
pub fn load(path: &str) -> Result<Config> {
    let config_path = std::path::Path::new(path);
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Config file not found: {}", path))?;
    let mut config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path))?;

    // config.toml の場所を基準に相対パスを絶対パスに展開
    let base_dir = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .canonicalize()
        .unwrap_or_else(|_| {
            // canonicalizeが失敗した場合はカレントディレクトリを使う
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    config.resolve_paths(&base_dir);
    validate(&config)?;
    Ok(config)
}

impl Config {
    fn resolve_paths(&mut self, base_dir: &std::path::Path) {
        self.rag.index_path  = resolve(base_dir, &self.rag.index_path);
        self.rag.chunks_path = resolve(base_dir, &self.rag.chunks_path);
        self.embedder.onnx_path      = resolve(base_dir, &self.embedder.onnx_path);
        self.embedder.tokenizer_path = resolve(base_dir, &self.embedder.tokenizer_path);
        self.conversation.summary_dir =
            resolve(base_dir, &self.conversation.summary_dir);
    }
}

fn resolve(base_dir: &std::path::Path, path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        path.to_string()
    } else {
        base_dir.join(p).to_string_lossy().to_string()
    }
}

/// Validate config values
fn validate(config: &Config) -> Result<()> {
    // ONNXモデルファイルの存在確認
    if !Path::new(&config.embedder.onnx_path).exists() {
        anyhow::bail!(
            "ONNX model not found: {}\nRun `rag init` to download the model.",
            config.embedder.onnx_path
        );
    }

    // tokenizerファイルの存在確認
    if !Path::new(&config.embedder.tokenizer_path).exists() {
        anyhow::bail!(
            "Tokenizer not found: {}\nRun `rag init` to download the model.",
            config.embedder.tokenizer_path
        );
    }

    // chunk_overlap は chunk_size より小さくなければならない
    if config.rag.chunk_overlap >= config.rag.chunk_size {
        anyhow::bail!(
            "chunk_overlap ({}) must be less than chunk_size ({})",
            config.rag.chunk_overlap,
            config.rag.chunk_size
        );
    }

    // top_k は 1 以上
    if config.rag.top_k == 0 {
        anyhow::bail!("top_k must be at least 1");
    }

    Ok(())
}

/// Load config without validation (for `rag init`)
pub fn load_without_validate(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Config file not found: {}", path))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path))?;

    Ok(config)
}
