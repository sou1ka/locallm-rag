//! chat command
//!
//! Interactive REPL-style chat session with conversation history.
//! Supports session continuity and RAG-integrated responses.

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use rag_core::{
    conversation::{Conversation, ConversationManager},
    embedder::Embedder,
    llm::{default_system_prompt, LlmClient},
    retriever::Retriever,
    store::Store,
};
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Run the chat command
pub async fn run(config: &Config, session_id: &str, use_rag: bool) -> Result<()> {
    println!("{}", "LOCALLM_RAG Chat".cyan().bold());
    println!(
        "{} Session: {} / Model: {} / RAG: {}",
        "ℹ".blue(),
        session_id.yellow(),
        config.llm.model.cyan(),
        if use_rag {
            "enabled".green().to_string()
        } else {
            "disabled".dimmed().to_string()
        }
    );
    println!("{} Type {} to exit.", "ℹ".blue(), "/quit".yellow());
    println!("{}", "─".repeat(50).dimmed());

    // コンポーネント初期化
    let embedder = Embedder::new(config.embedder.clone())
        .map_err(|e| anyhow::anyhow!("Failed to init embedder: {}", e))?;

    let llm = LlmClient::new(config.llm.clone())
        .map_err(|e| anyhow::anyhow!("Failed to init LLM client: {}", e))?;

    let store = load_store(config)?;

    if use_rag && store.is_empty() {
        println!(
            "{} Index is empty. RAG disabled for this session.",
            "⚠".yellow()
        );
    }

    // セッション初期化
    let conversation = Conversation::new(session_id.to_string());
    let mut manager = ConversationManager::new(
        conversation,
        config.conversation.clone(),
        llm,
    );

    // REPLループ
    let stdin = io::stdin();
    loop {
        // プロンプト表示
        print!("{} ", "You:".green().bold());
        io::stdout().flush()?;

        // 入力読み取り
        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        // 終了コマンド
        match input.as_str() {
            "/quit" | "/exit" | "/q" => {
                println!("{} Goodbye!", "👋".cyan());
                break;
            }
            "/help" => {
                print_help();
                continue;
            }
            "/history" => {
                print_history(&manager);
                continue;
            }
            "/clear" => {
                manager.conversation = Conversation::new(session_id.to_string());
                println!("{} Conversation cleared.", "✓".green());
                continue;
            }
            "/stats" => {
                print_session_stats(&manager, &store);
                continue;
            }
            _ => {}
        }

        // RAG 検索
        let rag_results = if use_rag && !store.is_empty() {
            match embedder.embed_query(&input) {
                Ok(embedding) => {
                    let retriever = Retriever::new(&store, config.rag.clone());
                    match retriever.retrieve(&embedding) {
                        Ok(results) => {
                            if !results.is_empty() {
                                println!(
                                    "{} {} chunk(s) retrieved.",
                                    "↑".dimmed(),
                                    results.len()
                                );
                            }
                            results
                        }
                        Err(e) => {
                            eprintln!("{} Retrieval error: {}", "⚠".yellow(), e);
                            vec![]
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{} Embedding error: {}", "⚠".yellow(), e);
                    vec![]
                }
            }
        } else {
            vec![]
        };

        // LLM 呼び出し（ストリーミング）
        println!("{}", "Assistant:".cyan().bold());

        let result = manager
            .chat_stream(
                &input,
                &rag_results,
                default_system_prompt(),
                |token| {
                    print!("{}", token);
                    io::stdout().flush().ok();
                },
            )
            .await;

        println!();

        match result {
            Ok(_) => {
                // 要約圧縮が発生したか表示
                if manager.conversation.summary.is_some()
                    && manager.conversation.turn_count()
                        == config.conversation.summary_keep_recent
                {
                    println!(
                        "{} Conversation compressed into summary.",
                        "↓".dimmed()
                    );
                }
            }
            Err(e) => {
                eprintln!("{} Error: {}", "✗".red(), e);
            }
        }

        println!("{}", "─".repeat(50).dimmed());
    }

    Ok(())
}

/// Store のロード
fn load_store(config: &Config) -> Result<Store> {
    let chunks_path = &config.rag.chunks_path;

    if Path::new(chunks_path).exists() {
        Store::load(
            config.rag.clone(),
            &config.rag.index_path,
            chunks_path,
        )
        .map_err(|e| anyhow::anyhow!("Failed to load store: {}", e))
    } else {
        Store::new(
            config.rag.clone(),
            &config.rag.index_path,
            chunks_path,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create store: {}", e))
    }
}

/// ヘルプ表示
fn print_help() {
    println!();
    println!("{}", "Commands:".cyan().bold());
    println!("  {}    Exit the chat", "/quit".yellow());
    println!("  {}    Show conversation history", "/history".yellow());
    println!("  {}    Clear conversation history", "/clear".yellow());
    println!("  {}    Show session statistics", "/stats".yellow());
    println!("  {}    Show this help", "/help".yellow());
    println!();
}

/// 会話履歴表示
fn print_history(manager: &ConversationManager) {
    println!();
    println!("{}", "Conversation History".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    if let Some(summary) = &manager.conversation.summary {
        println!("{}", "[Summary]".dimmed());
        println!("{}", summary.dimmed());
        println!("{}", "─".repeat(50).dimmed());
    }

    if manager.conversation.messages.is_empty() {
        println!("{}", "No messages yet.".dimmed());
    } else {
        for msg in &manager.conversation.messages {
            match msg.role.as_str() {
                "user" => println!(
                    "{} {}",
                    "You:".green().bold(),
                    msg.content
                ),
                "assistant" => println!(
                    "{} {}",
                    "Assistant:".cyan().bold(),
                    msg.content
                ),
                _ => {}
            }
            println!("{}", "─".repeat(50).dimmed());
        }
    }
    println!();
}

/// セッション統計表示
fn print_session_stats(manager: &ConversationManager, store: &Store) {
    println!();
    println!("{}", "Session Stats".cyan().bold());
    println!("{}", "─".repeat(30).dimmed());
    println!(
        "  Session ID  : {}",
        manager.conversation.id.yellow()
    );
    println!(
        "  Title       : {}",
        manager
            .conversation
            .title
            .as_deref()
            .unwrap_or("(untitled)")
            .cyan()
    );
    println!(
        "  Turns       : {}",
        manager.conversation.turn_count().to_string().cyan()
    );
    println!(
        "  Summary     : {}",
        if manager.conversation.summary.is_some() {
            "yes".green().to_string()
        } else {
            "no".dimmed().to_string()
        }
    );
    println!(
        "  Index chunks: {}",
        store.len().to_string().cyan()
    );
    println!("{}", "─".repeat(30).dimmed());
    println!();
}
