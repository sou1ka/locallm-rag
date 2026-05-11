//! Shared application state

use crate::config::Config;
use anyhow::Result;
use rag_core::{embedder::Embedder, llm::LlmClient, store::Store};
use std::sync::{Arc, Mutex};

pub struct AppStateInner {
    pub config: Config,
    pub store: Store,
    pub embedder: Embedder,
    pub llm: Arc<LlmClient>,
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

        let inner = AppStateInner { config, store, embedder, llm };
        Ok(AppState(Arc::new(Mutex::new(inner))))
    }
}
