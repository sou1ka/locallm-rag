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
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Config file not found: {}", path))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path))?;

    validate(&config)?;

    Ok(config)
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
