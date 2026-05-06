//! Daemon configuration loader

use rag_core::config::{ConversationConfig, EmbedderConfig, LlmConfig, OcrConfig, RagConfig};
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
    pub sources: Vec<SourceConfig>,
}

/// Source definition in config.toml
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

impl Config {
    /// Load config from toml file
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config.toml: {}", e))?;
        Ok(config)
    }

    /// Load from default path (./config.toml)
    pub fn load_default() -> anyhow::Result<Self> {
        Self::load("config.toml")
    }
}
