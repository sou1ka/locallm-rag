//! query command
//!
//! Single-shot query through the RAG pipeline.
//! Retrieves relevant chunks and sends to Ollama for an answer.

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rag_core::{
    embedder::Embedder,
    llm::{build_rag_prompt, default_system_prompt, LlmClient},
    retriever::Retriever,
    store::Store,
};
use std::path::Path;

/// Run the query command
pub async fn run(
    config: &Config,
    text: &str,
    use_rag: bool,
    top_k: Option<usize>,
    show_context: bool,
) -> Result<()> {
    // Store のロード
    let mut rag_config = config.rag.clone();
    if let Some(k) = top_k {
        rag_config.top_k = k;
    }

    let store = load_store(config)?;

    if use_rag && store.is_empty() {
        println!(
            "{} Index is empty. Run `rag ingest <path>` first.",
            "⚠".yellow()
        );
        println!("{} Falling back to direct LLM query.", "→".cyan());
    }

    // Embedder 初期化
    let embedder = Embedder::new(config.embedder.clone())
        .map_err(|e| anyhow::anyhow!("Failed to init embedder: {}", e))?;

    // LLM クライアント初期化
    let llm = LlmClient::new(config.llm.clone())
        .map_err(|e| anyhow::anyhow!("Failed to init LLM client: {}", e))?;

    // RAG 検索
    let rag_results = if use_rag && !store.is_empty() {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message("Searching index...");

        let query_embedding = embedder
            .embed_query(text)
            .map_err(|e| anyhow::anyhow!("Embedding failed: {}", e))?;

        let retriever = Retriever::new(&store, rag_config);
        let results = retriever
            .retrieve(&query_embedding)
            .map_err(|e| anyhow::anyhow!("Retrieval failed: {}", e))?;

        spinner.finish_and_clear();

        if results.is_empty() {
            println!(
                "{} No relevant chunks found (score threshold: {}).",
                "⚠".yellow(),
                config.rag.score_threshold
            );
        } else {
            println!(
                "{} Found {} relevant chunk(s).",
                "✓".green(),
                results.len()
            );
        }

        results
    } else {
        vec![]
    };

    // コンテキスト表示（--show-context）
    if show_context && !rag_results.is_empty() {
        println!();
        println!("{}", "RAG Context".cyan().bold());
        println!("{}", "─".repeat(40).dimmed());
        for (i, result) in rag_results.iter().enumerate() {
            println!(
                "{} {} (score: {:.3})",
                format!("[{}]", i + 1).yellow(),
                result.chunk.file_path.dimmed(),
                result.score
            );
            println!("{}", result.chunk.content.trim());
            println!("{}", "─".repeat(40).dimmed());
        }
        println!();
    }

    // コンテキスト文字列構築
    let context = rag_core::conversation::build_context(&rag_results);

    // プロンプト構築
    let messages = build_rag_prompt(
        &default_system_prompt(),
        &context,
        vec![],
        text,
    );

    // Ollama 呼び出し（ストリーミング）
    println!();
    println!("{}", "Answer".cyan().bold());
    println!("{}", "─".repeat(40).dimmed());

    let mut full_response = String::new();

    llm.complete_stream(messages, 0.7, 2048, |token| {
        print!("{}", token);
        use std::io::Write;
        std::io::stdout().flush().ok();
        full_response.push_str(&token);
    })
    .await
    .map_err(|e| anyhow::anyhow!("LLM request failed: {}", e))?;

    println!();
    println!("{}", "─".repeat(40).dimmed());

    Ok(())
}

fn load_store(config: &Config) -> Result<Store> {
    Store::open(config.rag.clone(), &config.rag.db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open store: {}", e))
}
