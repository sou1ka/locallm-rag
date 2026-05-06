//! chat command
//!
//! Interactive REPL-style chat session with conversation history.
//! Supports session continuity, history save/load, and RAG-integrated responses.

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use rag_core::{
    conversation::{Conversation, ConversationManager},
    embedder::Embedder,
    history::{HistoryManager,current_datetime},
    llm::{default_system_prompt, LlmClient},
    retriever::Retriever,
    store::Store,
};
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Run the chat command
pub async fn run(
    config: &Config,
    session: Option<&str>,
    use_rag: bool,
) -> Result<()> {
    let history = HistoryManager::new(&config.conversation.history_dir)?;

    // セッション指定があれば過去の会話を読み込む
    let (conversation, resumed) = match session {
        Some(session_id) => {
            match history.find_path(session_id) {
                Some(path) => {
                    match history.load(&path) {
                        Ok(conv) => {
                            println!(
                                "{} Resuming session: {}",
                                "↩".cyan(),
                                conv.title.as_deref().unwrap_or(session_id).yellow()
                            );
                            (conv, true)
                        }
                        Err(e) => {
                            eprintln!("{} Failed to load session: {}", "⚠".yellow(), e);
                            (Conversation::new(new_session_id()), false)
                        }
                    }
                }
                None => {
                    eprintln!(
                        "{} Session not found: {}. Starting new session.",
                        "⚠".yellow(),
                        session_id
                    );
                    (Conversation::new(new_session_id()), false)
                }
            }
        }
        None => (Conversation::new(new_session_id()), false),
    };

    println!("{}", "LOCALLM_RAG Chat".cyan().bold());
    println!(
        "{} Session: {} / Model: {} / RAG: {}",
        "ℹ".blue(),
        conversation.id.yellow(),
        config.llm.model.cyan(),
        if use_rag {
            "enabled".green().to_string()
        } else {
            "disabled".dimmed().to_string()
        }
    );
    println!(
        "{} Type {} to exit, {} for help.",
        "ℹ".blue(),
        "/quit".yellow(),
        "/help".yellow()
    );
    println!("{}", "─".repeat(50).dimmed());

    // 再開した場合は直近の履歴を表示
    if resumed && !conversation.messages.is_empty() {
        println!("{}", "Recent history:".dimmed());
        let recent = conversation.messages.iter().rev().take(6).collect::<Vec<_>>();
        for msg in recent.into_iter().rev() {
            match msg.role.as_str() {
                "user" => println!(
                    "{} {}",
                    "You:".green().bold(),
                    msg.content.chars().take(80).collect::<String>().dimmed()
                ),
                "assistant" => println!(
                    "{} {}",
                    "Assistant:".cyan().bold(),
                    msg.content.chars().take(80).collect::<String>().dimmed()
                ),
                _ => {}
            }
        }
        println!("{}", "─".repeat(50).dimmed());
    }

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

    let mut manager = ConversationManager::new(
        conversation,
        config.conversation.clone(),
        llm,
    );

    // REPLループ
    let stdin = io::stdin();
    loop {
        print!("{} ", "You:".green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim().to_string();

        if input.is_empty() {
            continue;
        }

        match input.as_str() {
            "/quit" | "/exit" | "/q" => {
                // 会話を保存
                save_session(&history, &manager.conversation);
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
                // クリア前に保存
                save_session(&history, &manager.conversation);
                manager.conversation = Conversation::new(new_session_id());
                println!("{} Conversation cleared and saved.", "✓".green());
                continue;
            }
            "/stats" => {
                print_session_stats(&manager, &store);
                continue;
            }
            "/save" => {
                save_session(&history, &manager.conversation);
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
                // 要約圧縮が発生した場合に通知
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

/// List past conversation sessions
pub async fn list_sessions(config: &Config) -> Result<()> {
    let history = HistoryManager::new(&config.conversation.history_dir)?;
    let entries = history.list()?;

    if entries.is_empty() {
        println!("{} No conversation history found.", "ℹ".blue());
        println!(
            "{} Start a chat with: {}",
            "ℹ".blue(),
            "rag chat".yellow()
        );
        return Ok(());
    }

    println!();
    println!("{}", "Conversation History".cyan().bold());
    println!("{}", "─".repeat(60).dimmed());
    println!(
        "{:<18} {}",
        "Session ID".dimmed(),
        "Title".dimmed()
    );
    println!("{}", "─".repeat(60).dimmed());

    for entry in &entries {
        println!(
            "{:<18} {}",
            entry.session_id.yellow(),
            entry.title.cyan()
        );
    }

    println!("{}", "─".repeat(60).dimmed());
    println!(
        "{} {} session(s) found.",
        "ℹ".blue(),
        entries.len()
    );
    println!();
    println!(
        "{} Resume a session: {}",
        "ℹ".blue(),
        "rag chat --session <session_id>".yellow()
    );

    Ok(())
}

/// Save session to history (with feedback)
fn save_session(history: &HistoryManager, conversation: &Conversation) {
    if conversation.messages.is_empty() {
        return;
    }
    match history.save(conversation) {
        Ok(path) => println!(
            "{} Session saved: {}",
            "✓".green(),
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .dimmed()
        ),
        Err(e) => eprintln!("{} Failed to save session: {}", "⚠".yellow(), e),
    }
}

/// Generate a new session ID (YYYYMMDD_HHMMSS format)
fn new_session_id() -> String {
    current_datetime()
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
    println!("  {}    Exit and save session", "/quit".yellow());
    println!("  {}    Save session manually", "/save".yellow());
    println!("  {}    Show conversation history", "/history".yellow());
    println!("  {}    Clear and save current session", "/clear".yellow());
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
