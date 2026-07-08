//! rag-server: OpenAI-compatible HTTP server for LAN sharing
//!
//! Exposes rag-core as a REST API compatible with the OpenAI chat completions format.
//! Suitable for connecting local apps, scripts, or other tools over the network.

mod config;
mod routes;
mod state;
mod ws;

use anyhow::Result;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = find_config();
    let config = config::Config::load(&config_path)?;

    // ログフィルタの優先順位: RUST_LOG 環境変数 > config.toml の server.log_filter > デフォルト
    let log_filter = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| config.server.log_filter.clone())
        .unwrap_or_else(|| "rag_server=info".into());

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(log_filter))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Loaded config from: {}", config_path);

    let bind_addr = format!("{}:{}", config.server.host, config.server.port);

    let app_state = state::AppState::init(config)?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::router(app_state).layer(cors).layer(TraceLayer::new_for_http());

    let addr: SocketAddr = bind_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid bind address '{}': {}", bind_addr, e))?;

    tracing::info!("rag-server listening on http://{}", addr);
    tracing::info!("OpenAI-compatible endpoint: http://{}/v1/chat/completions", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn find_config() -> String {
    if let Ok(path) = std::env::var("RAG_CONFIG") {
        return path;
    }

    // 実行ファイルの隣
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_dir = exe_path.parent().unwrap_or(std::path::Path::new("."));
        let candidate = exe_dir.join("config.toml");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }

    // カレントディレクトリから親を遡って探す
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        let candidate = dir.join("config.toml");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
        if !dir.pop() {
            break;
        }
    }

    "config.toml".to_string()
}
