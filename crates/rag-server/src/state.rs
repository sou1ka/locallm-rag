//! Shared application state

use crate::config::Config;
use anyhow::Result;
use rag_core::{embedder::Embedder, llm::LlmClient, store::Store};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub struct AppStateInner {
    pub config: Config,
    pub store: Store,
    pub embedder: Embedder,
    pub llm: Arc<LlmClient>,
    pub history_dir: PathBuf,
    /// TTSクライアント（オプション機能。Noneなら音声合成なし）
    pub tts: Option<Arc<tts_emotion::TtsClient>>,
    /// 感情抽出ルール（オプション機能。Noneなら感情抽出なし）
    pub emotion_rules: Option<Arc<tts_emotion::EmotionRules>>,
    /// キャラクター設定（オプション機能。Noneならキャラ切り替えなし）
    pub characters: Option<Arc<tts_emotion::CharacterStore>>,
}

#[derive(Clone)]
pub struct AppState(pub Arc<Mutex<AppStateInner>>);

impl AppState {
    pub fn init(config: Config) -> Result<Self> {
        let store = Store::open(config.rag.clone(), &config.rag.db_path)
            .map_err(|e| anyhow::anyhow!("Failed to open store: {}", e))?;

        let embedder = Embedder::new(config.embedder.clone())
            .map_err(|e| anyhow::anyhow!("Failed to init embedder: {}", e))?;

        let llm = Arc::new(
            LlmClient::new(config.llm.clone())
                .map_err(|e| anyhow::anyhow!("Failed to init LLM client: {}", e))?,
        );

        let history_dir = PathBuf::from(&config.conversation.history_dir);

        // TTS・感情抽出・キャラクターはオプション機能。設定が無い/読み込み失敗でもサーバーは起動する
        let tts = config
            .tts
            .clone()
            .map(|c| Arc::new(tts_emotion::TtsClient::new(c)));

        let emotion_rules = config.emotion.as_ref().and_then(|e| {
            match tts_emotion::EmotionRules::load(&e.rules_path) {
                Ok(rules) => Some(Arc::new(rules)),
                Err(err) => {
                    tracing::warn!("Failed to load emotion rules ({}): {}", e.rules_path, err);
                    None
                }
            }
        });

        let characters = config.chara.as_ref().and_then(|c| {
            match tts_emotion::CharacterStore::load_dir(&c.dir) {
                Ok(store) => {
                    tracing::info!("Loaded {} character(s): {:?}", store.len(), store.ids());
                    Some(Arc::new(store))
                }
                Err(err) => {
                    tracing::warn!("Failed to load characters ({}): {}", c.dir, err);
                    None
                }
            }
        });

        let inner = AppStateInner { config, store, embedder, llm, history_dir, tts, emotion_rules, characters };
        Ok(AppState(Arc::new(Mutex::new(inner))))
    }
}
