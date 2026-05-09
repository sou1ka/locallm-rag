//! Configuration module for RAG engine
//!
//! Loads and validates RAG configuration from TOML files.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Root configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rag: RagConfig,
    pub embedder: EmbedderConfig,
    pub ocr: OcrConfig,
    pub llm: LlmConfig,
    pub conversation: ConversationConfig,
    pub daemon: DaemonConfig,
    pub sources: Vec<SourceConfig>,
}

/// RAG engine specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    pub db_path: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub top_k: usize,
    pub score_threshold: f32,
}

/// Embedding model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedderConfig {
    pub model_type: String,
    pub onnx_path: String,
    pub tokenizer_path: String,
    pub doc_prefix: String,
    pub query_prefix: String,
    #[serde(default = "default_embed_batch_size")]
    pub embed_batch_size: usize,
}

fn default_embed_batch_size() -> usize {
    128
}

/// OCR settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub enabled: bool,
    pub tessdata_path: String,
    pub lang: String,
    pub min_dpi: u32,
}

/// LLM (Ollama) connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

/// Conversation history settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationConfig {
    pub max_history_turns: usize,
    pub summary_keep_recent: usize,
    pub auto_save_summary: bool,
    pub summary_dir: String,
    pub history_dir: String,
}

/// Daemon process settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub pid_path: String,
}

/// Data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    #[serde(rename = "type")]
    pub source_type: String,
    pub path: String,
    pub extensions: Option<Vec<String>>,
    pub tables: Option<Vec<String>>,

    // Audio-specific settings
    #[serde(default)]
    pub audio: Option<AudioConfig>,

    // Video-specific settings
    #[serde(default)]
    pub video: Option<VideoConfig>,

    // Image-specific settings
    #[serde(default)]
    pub image: Option<ImageConfig>,
}

/// Audio metadata field configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub fields: Option<Vec<String>>,
}

/// Video processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub use_ffprobe: Option<bool>,
    pub use_subtitle: Option<bool>,
}

/// Image processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    pub use_exif: Option<bool>,
    pub use_path_as_category: Option<bool>,
}

impl Config {
    /// Load configuration from a TOML file
    ///
    /// # Arguments
    /// * `path` - Path to the config.toml file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed as TOML
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            anyhow::anyhow!("Failed to read config file '{}': {}", path.display(), e)
        })?;
        Self::from_str(&content)
    }

    /// Parse configuration from TOML string
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: Config = toml::from_str(content).map_err(|e| {
            anyhow::anyhow!("Failed to parse TOML configuration: {}", e)
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Create configuration with default values (for testing)
    pub fn default_for_testing() -> Self {
        Self {
            rag: RagConfig {
                db_path: "./data/store.db".to_string(),
                chunk_size: 512,
                chunk_overlap: 128,
                top_k: 5,
                score_threshold: 0.5,
            },
            embedder: EmbedderConfig {
                model_type: "ruri-v3".to_string(),
                onnx_path: "./models/ruri-v3-310m.onnx".to_string(),
                tokenizer_path: "./models/tokenizer.json".to_string(),
                doc_prefix: "文章: ".to_string(),
                query_prefix: "クエリ: ".to_string(),
                embed_batch_size: 128,
            },
            ocr: OcrConfig {
                enabled: true,
                tessdata_path: "/usr/share/tessdata".to_string(),
                lang: "jpn".to_string(),
                min_dpi: 150,
            },
            llm: LlmConfig {
                base_url: "http://localhost:11434/v1".to_string(),
                model: "mistral".to_string(),
                api_key: "ollama".to_string(),
            },
            conversation: ConversationConfig {
                max_history_turns: 20,
                summary_keep_recent: 5,
                auto_save_summary: true,
                summary_dir: "./data/summaries".to_string(),
                history_dir: "./history".to_string(),
            },
            daemon: DaemonConfig {
                socket_path: "/tmp/rag-daemon.sock".to_string(),
                pid_path: "/tmp/rag-daemon.pid".to_string(),
            },
            sources: vec![],
        }
    }

    /// Validate configuration values
    fn validate(&self) -> anyhow::Result<()> {
        // Validate RAG settings
        if self.rag.chunk_size == 0 {
            return Err(anyhow::anyhow!("rag.chunk_size must be greater than 0"));
        }
        if self.rag.chunk_overlap >= self.rag.chunk_size {
            return Err(anyhow::anyhow!(
                "rag.chunk_overlap must be less than rag.chunk_size"
            ));
        }
        if self.rag.top_k == 0 {
            return Err(anyhow::anyhow!("rag.top_k must be greater than 0"));
        }
        if self.rag.score_threshold < 0.0 || self.rag.score_threshold > 1.0 {
            return Err(anyhow::anyhow!(
                "rag.score_threshold must be between 0.0 and 1.0"
            ));
        }

        // Validate embedder settings
        if self.embedder.model_type.is_empty() {
            return Err(anyhow::anyhow!("embedder.model_type must not be empty"));
        }
        if self.embedder.onnx_path.is_empty() {
            return Err(anyhow::anyhow!("embedder.onnx_path must not be empty"));
        }
        if self.embedder.tokenizer_path.is_empty() {
            return Err(anyhow::anyhow!(
                "embedder.tokenizer_path must not be empty"
            ));
        }

        // Validate OCR settings
        if self.ocr.enabled {
            if self.ocr.tessdata_path.is_empty() {
                return Err(anyhow::anyhow!("ocr.tessdata_path must not be empty"));
            }
            if self.ocr.lang.is_empty() {
                return Err(anyhow::anyhow!("ocr.lang must not be empty"));
            }
        }

        // Validate LLM settings
        if self.llm.base_url.is_empty() {
            return Err(anyhow::anyhow!("llm.base_url must not be empty"));
        }
        if self.llm.model.is_empty() {
            return Err(anyhow::anyhow!("llm.model must not be empty"));
        }

        // Validate conversation settings
        if self.conversation.max_history_turns == 0 {
            return Err(anyhow::anyhow!(
                "conversation.max_history_turns must be greater than 0"
            ));
        }
        if self.conversation.summary_keep_recent > self.conversation.max_history_turns {
            return Err(anyhow::anyhow!(
                "conversation.summary_keep_recent must be <= max_history_turns"
            ));
        }

        Ok(())
    }

    /// Expand path with ~ to home directory
    pub fn expand_path(path: &str) -> PathBuf {
        if path.starts_with("~") {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(path.replace("~", &home));
            } else if let Ok(home) = std::env::var("USERPROFILE") {
                // Windows
                return PathBuf::from(path.replace("~", &home));
            }
        }
        PathBuf::from(path)
    }

    /// Get absolute path for daemon socket
    pub fn get_socket_path(&self) -> PathBuf {
        Self::expand_path(&self.daemon.socket_path)
    }

    /// Get absolute path for daemon PID file
    pub fn get_pid_path(&self) -> PathBuf {
        Self::expand_path(&self.daemon.pid_path)
    }

    /// Get absolute path for database
    pub fn get_db_path(&self) -> PathBuf {
        Self::expand_path(&self.rag.db_path)
    }

    /// Get absolute path for summaries directory
    pub fn get_summary_dir(&self) -> PathBuf {
        Self::expand_path(&self.conversation.summary_dir)
    }

    /// Get sources by type
    pub fn get_sources_by_type(&self, source_type: &str) -> Vec<&SourceConfig> {
        self.sources
            .iter()
            .filter(|s| s.source_type == source_type)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default_for_testing();
        assert_eq!(config.rag.chunk_size, 512);
        assert_eq!(config.rag.top_k, 5);
        assert!(config.ocr.enabled);
    }

    #[test]
    fn test_expand_path() {
        let path = Config::expand_path("./data");
        assert_eq!(path, PathBuf::from("./data"));
    }

    #[test]
    fn test_invalid_chunk_overlap() {
        let toml_str = r#"
[rag]
db_path = "./data/lancedb_data"
chunk_size = 100
chunk_overlap = 150
top_k = 5
score_threshold = 0.5

[embedder]
model_type = "ruri-v3"
onnx_path = "./models/ruri.onnx"
tokenizer_path = "./models/tokenizer.json"
doc_prefix = "文章: "
query_prefix = "クエリ: "

[ocr]
enabled = true
tessdata_path = "/usr/share/tessdata"
lang = "jpn"
min_dpi = 150

[llm]
base_url = "http://localhost:11434/v1"
model = "mistral"
api_key = "ollama"

[conversation]
max_history_turns = 20
summary_keep_recent = 5
auto_save_summary = true
summary_dir = "./data/summaries"

[daemon]
socket_path = "/tmp/rag-daemon.sock"
pid_path = "/tmp/rag-daemon.pid"
"#;

        let result = Config::from_str(toml_str);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("chunk_overlap"));
    }

    #[test]
    fn test_valid_config_parse() {
        let toml_str = r#"
[rag]
db_path = "./data/lancedb_data"
chunk_size = 512
chunk_overlap = 128
top_k = 5
score_threshold = 0.75

[embedder]
model_type = "ruri-v3"
onnx_path = "./models/ruri.onnx"
tokenizer_path = "./models/tokenizer.json"
doc_prefix = "文章: "
query_prefix = "クエリ: "

[ocr]
enabled = true
tessdata_path = "/usr/share/tessdata"
lang = "jpn"
min_dpi = 300

[llm]
base_url = "http://localhost:11434/v1"
model = "llama3.2"
api_key = "ollama"

[conversation]
max_history_turns = 20
summary_keep_recent = 5
auto_save_summary = true
summary_dir = "./data/summaries"

[daemon]
socket_path = "/tmp/rag-daemon.sock"
pid_path = "/tmp/rag-daemon.pid"

[[sources]]
type = "directory"
path = "./docs"
extensions = ["md", "txt"]
"#;

        let config = Config::from_str(toml_str);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.rag.chunk_size, 512);
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].source_type, "directory");
    }
}
