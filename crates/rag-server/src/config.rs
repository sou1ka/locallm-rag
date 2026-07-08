//! rag-server configuration

use anyhow::{Context, Result};
use rag_core::config::{ConversationConfig, EmbedderConfig, LlmConfig, RagConfig};
use serde::Deserialize;
use std::path::Path;

/// Top-level config matching config.toml
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub rag: RagConfig,
    pub embedder: EmbedderConfig,
    pub llm: LlmConfig,
    pub conversation: ConversationConfig,
    #[serde(default)]
    pub server: ServerConfig,
    /// TTS設定（オプション機能。未設定なら音声合成なしで動作）
    #[serde(default)]
    pub tts: Option<tts_emotion::TtsConfig>,
    /// 感情抽出設定（オプション機能。未設定なら感情抽出なしで動作）
    #[serde(default)]
    pub emotion: Option<EmotionConfig>,
    /// キャラクター設定（オプション機能。未設定ならキャラ切り替えなしで動作）
    #[serde(default)]
    pub chara: Option<CharaConfig>,
}

/// Emotion extraction settings
#[derive(Debug, Clone, Deserialize)]
pub struct EmotionConfig {
    /// emotion_rules.toml のパス
    pub rules_path: String,
}

/// Character settings
#[derive(Debug, Clone, Deserialize)]
pub struct CharaConfig {
    /// キャラ設定tomlを置くディレクトリ
    pub dir: String,
    /// デフォルトで使用するキャラID（省略時はリクエストで chara 未指定なら無指定扱い）
    #[serde(default)]
    pub default: Option<String>,
}

/// HTTP server settings
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind address (default: 127.0.0.1)
    #[serde(default = "default_host")]
    pub host: String,
    /// Bind port (default: 18080)
    #[serde(default = "default_port")]
    pub port: u16,
    /// Bearer token for API auth. Empty string disables auth.
    #[serde(default)]
    pub api_key: String,
    /// ログフィルタ（例: "rag_server=info,tower_http=debug"）。
    /// RUST_LOG 環境変数が設定されている場合はそちらが優先される。
    #[serde(default)]
    pub log_filter: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            api_key: String::new(),
            log_filter: None,
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    18080
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let config_path = Path::new(path);
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("Config file not found: {}", path))?;

        let mut config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        let base_dir = config_path
            .parent()
            .unwrap_or(Path::new("."))
            .canonicalize()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| ".".into()));

        config.resolve_paths(&base_dir);
        Ok(config)
    }

    fn resolve_paths(&mut self, base_dir: &Path) {
        self.rag.db_path             = resolve(base_dir, &self.rag.db_path);
        self.embedder.onnx_path      = resolve(base_dir, &self.embedder.onnx_path);
        self.embedder.tokenizer_path = resolve(base_dir, &self.embedder.tokenizer_path);
        self.conversation.summary_dir = resolve(base_dir, &self.conversation.summary_dir);
        self.conversation.history_dir = resolve(base_dir, &self.conversation.history_dir);
        if let Some(emotion) = &mut self.emotion {
            emotion.rules_path = resolve(base_dir, &emotion.rules_path);
        }
        if let Some(chara) = &mut self.chara {
            chara.dir = resolve(base_dir, &chara.dir);
        }
    }
}

fn resolve(base_dir: &Path, path: &str) -> String {
    let p = Path::new(path);
    if p.is_absolute() {
        path.to_string()
    } else {
        base_dir.join(p).to_string_lossy().to_string()
    }
}
