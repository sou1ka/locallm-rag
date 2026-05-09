//! ingest command
//!
//! Ingests files from a path into the vector index.
//! Supports single file, directory (recursive), and watch mode.

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use rag_core::{
    chunker::ChunkSplitter,
    embedder::Embedder,
    ingestor,
    store::Store,
};
use std::path::Path;

/// Run the ingest command
pub async fn run(
    config: &Config,
    path: &str,
    extensions: Option<Vec<String>>,
    watch: bool,
) -> Result<()> {
    let target = Path::new(path);

    if !target.exists() {
        anyhow::bail!("Path not found: {}", path);
    }

    // 初回インジェスト実行
    ingest_once(config, target, extensions.as_deref()).await?;

    // watchモード
    if watch {
        println!("{} Watching for changes in: {}", "👁".cyan(), path);
        watch_mode(config, target, extensions.as_deref()).await?;
    }

    Ok(())
}

/// 一回分のインジェスト処理
async fn ingest_once(
    config: &Config,
    path: &Path,
    extensions: Option<&[String]>,
) -> Result<()> {
    println!("{} Ingesting: {}", "→".cyan(), path.display());

    // コンポーネント初期化
    let chunker = ChunkSplitter::new(config.rag.clone())
        .map_err(|e| anyhow::anyhow!("Failed to init chunker: {}", e))?;

    let embedder = Embedder::new(config.embedder.clone())
        .map_err(|e| anyhow::anyhow!("Failed to init embedder: {}", e))?;

    // Storeのロード or 新規作成
    let mut store = load_or_create_store(config)?;

    // ファイル走査・チャンク生成
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Scanning files...");

    let chunks = if path.is_dir() {
        ingestor::ingest_directory(path, extensions, &config.rag)
            .map_err(|e| anyhow::anyhow!("Ingest failed: {}", e))?
    } else {
        ingestor::ingest_file(path, &config.rag)
            .map_err(|e| anyhow::anyhow!("Ingest failed: {}", e))?
    };

    if chunks.is_empty() {
        spinner.finish_with_message("No chunks extracted.".yellow().to_string());
        return Ok(());
    }

    spinner.set_message(format!("Embedding {} chunks...", chunks.len()));
    spinner.finish_and_clear();

    let pb = ProgressBar::new(chunks.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} chunks {msg}")
            .unwrap()
            .progress_chars("█░"),
    );

    // バッチEmbedding → Store追加
    let mut files_processed = std::collections::HashSet::new();
    let skipped = 0usize;

    let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = embedder
        .embed_documents(texts)
        .map_err(|e| anyhow::anyhow!("Embedding failed: {}", e))?;

    for (mut chunk, embedding) in chunks.into_iter().zip(embeddings) {
        chunk.id = store.len();
        files_processed.insert(chunk.file_path.clone());
        store
            .add_chunk(embedding, chunk)
            .map_err(|e| anyhow::anyhow!("Store error: {}", e))?;
        pb.inc(1);
    }

    pb.finish_and_clear();

    // 保存
    store
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save index: {}", e))?;

    // 結果表示
    print_stats(&store, files_processed.len(), skipped);

    Ok(())
}

/// watchモード：ファイル変更を監視して自動再インジェスト
async fn watch_mode(
    config: &Config,
    path: &Path,
    extensions: Option<&[String]>,
) -> Result<()> {
    use notify::{Event, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    println!("{} Press Ctrl+C to stop watching.", "ℹ".blue());

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                // 書き込み・作成・削除イベントのみ反応
                use notify::EventKind;
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        // 対象拡張子のファイルかチェック
                        let relevant = event.paths.iter().any(|p| {
                            let ext = p
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            match extensions {
                                Some(exts) => exts.iter().any(|e| e.to_lowercase() == ext),
                                None => true,
                            }
                        });

                        if relevant {
                            println!(
                                "{} Change detected: {:?}",
                                "↻".yellow(),
                                event.paths
                            );
                            if let Err(e) = ingest_once(config, path, extensions).await {
                                eprintln!("{} Re-ingest failed: {}", "✗".red(), e);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Err(e)) => eprintln!("{} Watch error: {}", "✗".red(), e),
            Err(_) => {} // タイムアウト（正常）
        }
    }
}

fn load_or_create_store(config: &Config) -> Result<Store> {
    Store::open(config.rag.clone(), &config.rag.db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open store: {}", e))
}

/// インジェスト結果の表示
fn print_stats(store: &Store, files_processed: usize, files_skipped: usize) {
    println!();
    println!("{}", "─".repeat(40).dimmed());
    println!("{}", "Ingest complete".green().bold());
    println!(
        "  Files processed : {}",
        files_processed.to_string().cyan()
    );
    if files_skipped > 0 {
        println!(
            "  Files skipped   : {}",
            files_skipped.to_string().yellow()
        );
    }
    println!(
        "  Total chunks    : {}",
        store.len().to_string().cyan()
    );

    // ソース種別ごとの内訳
    let mut by_type: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for chunk in store.chunks() {
        *by_type.entry(chunk.source_type.clone()).or_insert(0) += 1;
    }
    let mut by_type: Vec<(String, usize)> = by_type.into_iter().collect();
    by_type.sort_by(|a, b| b.1.cmp(&a.1));

    if !by_type.is_empty() {
        println!();
        for (source_type, count) in &by_type {
            let bar_len = (*count * 20 / store.len()).max(1);
            let bar = "█".repeat(bar_len) + &"░".repeat(20 - bar_len);
            println!(
                "  {:<12} {}  {} chunks",
                source_type.cyan(),
                bar.blue(),
                count
            );
        }
    }
    println!("{}", "─".repeat(40).dimmed());
}
