//! index command
//!
//! Manages the vector index: stats and reset.

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use rag_core::store::Store;
use std::path::Path;

/// Show index statistics
pub async fn stats(config: &Config) -> Result<()> {
    let store = load_store(config)?;

    if store.is_empty() {
        println!("{} Index is empty. Run `rag ingest <path>` first.", "⚠".yellow());
        return Ok(());
    }

    // ソース種別ごとの集計
    let mut by_type: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut by_file: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for chunk in store.chunks() {
        *by_type.entry(chunk.source_type.clone()).or_insert(0) += 1;
        *by_file.entry(chunk.file_path.clone()).or_insert(0) += 1;
    }

    let mut by_type: Vec<(String, usize)> = by_type.into_iter().collect();
    by_type.sort_by(|a, b| b.1.cmp(&a.1));

    let total = store.len();

    // 表示
    println!();
    println!("{}", "Index Statistics".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
    println!(
        "  Total chunks : {}",
        total.to_string().cyan().bold()
    );
    println!(
        "  Total files  : {}",
        by_file.len().to_string().cyan()
    );
    println!(
        "  Index path   : {}",
        config.rag.index_path.dimmed()
    );
    println!(
        "  Chunks path  : {}",
        config.rag.chunks_path.dimmed()
    );
    println!();
    println!("{}", "By source type:".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    for (source_type, count) in &by_type {
        let ratio = *count as f32 / total as f32;
        let bar_filled = (ratio * 30.0) as usize;
        let bar_empty = 30 - bar_filled;
        let bar = format!(
            "{}{}",
            "█".repeat(bar_filled).blue(),
            "░".repeat(bar_empty).dimmed()
        );

        // ソース種別ごとのファイル数を集計
        let file_count = by_file
            .keys()
            .filter(|f| {
                store.chunks().iter().any(|c| {
                    &c.file_path == *f && &c.source_type == source_type
                })
            })
            .count();

        println!(
            "  {:<12} {}  {:>5} chunks  ({} files)",
            source_type.cyan(),
            bar,
            count,
            file_count
        );
    }

    println!("{}", "─".repeat(50).dimmed());
    println!();

    Ok(())
}

/// Reset the index
pub async fn reset(
    config: &Config,
    source: Option<&str>,
    yes: bool,
) -> Result<()> {
    match source {
        Some(source_path) => reset_source(config, source_path, yes).await,
        None => reset_all(config, yes).await,
    }
}

/// Reset entire index
async fn reset_all(config: &Config, yes: bool) -> Result<()> {
    if !yes {
        print!(
            "{} This will delete the entire index. Continue? [y/N] ",
            "⚠".yellow()
        );
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("{} Aborted.", "✗".red());
            return Ok(());
        }
    }

    // ファイル削除
    let index_path = &config.rag.index_path;
    let chunks_path = &config.rag.chunks_path;

    if Path::new(index_path).exists() {
        std::fs::remove_file(index_path)
            .map_err(|e| anyhow::anyhow!("Failed to delete index: {}", e))?;
    }
    if Path::new(chunks_path).exists() {
        std::fs::remove_file(chunks_path)
            .map_err(|e| anyhow::anyhow!("Failed to delete chunks: {}", e))?;
    }

    println!("{} Index reset complete.", "✓".green());
    Ok(())
}

/// Reset chunks from a specific source path only
async fn reset_source(config: &Config, source_path: &str, yes: bool) -> Result<()> {
    let mut store = load_store(config)?;

    // 対象チャンク数を確認
    let target_count = store
        .chunks()
        .iter()
        .filter(|c| c.file_path.starts_with(source_path))
        .count();

    if target_count == 0 {
        println!(
            "{} No chunks found for source: {}",
            "⚠".yellow(),
            source_path
        );
        return Ok(());
    }

    if !yes {
        print!(
            "{} Delete {} chunks from '{}'? [y/N] ",
            "⚠".yellow(),
            target_count,
            source_path
        );
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("{} Aborted.", "✗".red());
            return Ok(());
        }
    }

    // 対象以外のチャンクとembeddingを残す
    // store.retain_chunks() がないため再構築する
    let kept: Vec<rag_core::store::Chunk> = store
        .chunks()
        .iter()
        .filter(|c| !c.file_path.starts_with(source_path))
        .cloned()
        .collect();

    let removed = store.len() - kept.len();

    // Storeを作り直して保存
    let mut new_store = Store::new(
        config.rag.clone(),
        &config.rag.index_path,
        &config.rag.chunks_path,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create store: {}", e))?;

    // embeddingが取得できないためこの実装は部分的
    // 完全な実装はstore.rsにretain機能を追加する必要がある
    // 今はチャンクJSONだけ更新してembeddingは再インジェストで対応
    drop(new_store);

    // chunks.jsonだけ直接書き換える
    let json = serde_json::to_string_pretty(&kept)
        .map_err(|e| anyhow::anyhow!("Failed to serialize: {}", e))?;
    std::fs::write(&config.rag.chunks_path, json)
        .map_err(|e| anyhow::anyhow!("Failed to write chunks: {}", e))?;

    // index.binは削除して再インジェスト時に再構築
    if Path::new(&config.rag.index_path).exists() {
        std::fs::remove_file(&config.rag.index_path)
            .map_err(|e| anyhow::anyhow!("Failed to delete index: {}", e))?;
    }

    println!(
        "{} Removed {} chunks from '{}'.",
        "✓".green(),
        removed,
        source_path
    );
    println!(
        "{} Run `rag ingest` to rebuild the index.",
        "ℹ".blue()
    );

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
