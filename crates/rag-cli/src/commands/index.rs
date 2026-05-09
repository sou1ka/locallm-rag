//! index command — インデックス統計と管理

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use rag_core::store::Store;

pub async fn stats(config: &Config) -> Result<()> {
    let store = open_store(config)?;

    if store.is_empty() {
        println!("{} Index is empty. Run `rag ingest <path>` first.", "⚠".yellow());
        return Ok(());
    }

    let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut by_file: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for chunk in store.chunks() {
        *by_type.entry(chunk.source_type.clone()).or_insert(0) += 1;
        *by_file.entry(chunk.file_path.clone()).or_insert(0) += 1;
    }

    let mut by_type: Vec<(String, usize)> = by_type.into_iter().collect();
    by_type.sort_by(|a, b| b.1.cmp(&a.1));
    let total = store.len();

    println!();
    println!("{}", "Index Statistics".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());
    println!("  Total chunks : {}", total.to_string().cyan().bold());
    println!("  Total files  : {}", by_file.len().to_string().cyan());
    println!("  DB path      : {}", config.rag.db_path.dimmed());
    println!();
    println!("{}", "By source type:".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    for (source_type, count) in &by_type {
        let ratio = *count as f32 / total as f32;
        let filled = (ratio * 30.0) as usize;
        let bar = format!("{}{}", "█".repeat(filled).blue(), "░".repeat(30 - filled).dimmed());
        let file_count = by_file.keys()
            .filter(|f| store.chunks().iter().any(|c| &c.file_path == *f && &c.source_type == source_type))
            .count();
        println!("  {:<12} {}  {:>5} chunks  ({} files)", source_type.cyan(), bar, count, file_count);
    }

    println!("{}", "─".repeat(50).dimmed());
    println!();
    Ok(())
}

pub async fn reset(config: &Config, source: Option<&str>, yes: bool) -> Result<()> {
    match source {
        Some(src) => reset_source(config, src, yes).await,
        None => reset_all(config, yes).await,
    }
}

async fn reset_all(config: &Config, yes: bool) -> Result<()> {
    if !yes {
        print!("{} This will delete the entire index. Continue? [y/N] ", "⚠".yellow());
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut input)?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("{} Aborted.", "✗".red());
            return Ok(());
        }
    }

    open_store(config)?.clear()
        .map_err(|e| anyhow::anyhow!("Failed to reset index: {}", e))?;

    println!("{} Index reset complete.", "✓".green());
    Ok(())
}

async fn reset_source(config: &Config, source_path: &str, yes: bool) -> Result<()> {
    let mut store = open_store(config)?;

    let target_count = store.chunks().iter()
        .filter(|c| c.file_path.starts_with(source_path))
        .count();

    if target_count == 0 {
        println!("{} No chunks found for source: {}", "⚠".yellow(), source_path);
        return Ok(());
    }

    if !yes {
        print!("{} Delete {} chunks from '{}'? [y/N] ", "⚠".yellow(), target_count, source_path);
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut input)?;
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("{} Aborted.", "✗".red());
            return Ok(());
        }
    }

    let removed = store.delete_by_path(source_path)
        .map_err(|e| anyhow::anyhow!("Failed to delete chunks: {}", e))?;

    println!("{} Removed {} chunks from '{}'.", "✓".green(), removed, source_path);
    Ok(())
}

fn open_store(config: &Config) -> Result<Store> {
    Store::open(config.rag.clone(), &config.rag.db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open store: {}", e))
}
