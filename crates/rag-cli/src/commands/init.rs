//! init command
//!
//! Downloads ruri-v3 ONNX model files from HuggingFace.
//! Required before first use of the RAG engine.

use crate::config::Config;
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Model files to download
struct ModelFile {
    filename: &'static str,
    url: &'static str,
}

const MODEL_FILES: &[ModelFile] = &[
    ModelFile {
        filename: "model.onnx",
        url: "https://huggingface.co/keitokei1994/ruri-v3-310m-onnx/resolve/main/model.onnx",
    },
    ModelFile {
        filename: "tokenizer.json",
        url: "https://huggingface.co/keitokei1994/ruri-v3-310m-onnx/resolve/main/tokenizer.json",
    },
    ModelFile {
        filename: "config.json",
        url: "https://huggingface.co/keitokei1994/ruri-v3-310m-onnx/resolve/main/config.json",
    },
    ModelFile {
        filename: "special_tokens_map.json",
        url: "https://huggingface.co/keitokei1994/ruri-v3-310m-onnx/resolve/main/special_tokens_map.json",
    },
    ModelFile {
        filename: "tokenizer_config.json",
        url: "https://huggingface.co/keitokei1994/ruri-v3-310m-onnx/resolve/main/tokenizer_config.json",
    },
];

/// Run the init command
pub async fn run(config: &Config) -> Result<()> {
    println!("{}", "LOCALLM_RAG Init".cyan().bold());
    println!("{}", "─".repeat(50).dimmed());

    // モデルディレクトリの確認・作成
    let onnx_path = Path::new(&config.embedder.onnx_path);
    let model_dir = onnx_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid onnx_path in config"))?;

    std::fs::create_dir_all(model_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create model directory: {}", e))?;

    println!(
        "{} Model directory: {}",
        "ℹ".blue(),
        model_dir.display()
    );
    println!();

    // 各ファイルのダウンロード
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 大きいファイルのため10分
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    for model_file in MODEL_FILES {
        let dest = model_dir.join(model_file.filename);

        // 既存ファイルはスキップ
        if dest.exists() {
            println!(
                "{} {} (already exists, skipping)",
                "✓".green(),
                model_file.filename
            );
            continue;
        }

        println!(
            "{} Downloading {}...",
            "↓".cyan(),
            model_file.filename.yellow()
        );

        download_file(&client, model_file.url, &dest).await?;

        println!(
            "{} {} downloaded.",
            "✓".green(),
            model_file.filename
        );
    }

    println!();
    println!("{}", "─".repeat(50).dimmed());
    println!("{} Initialization complete.", "✓".green().bold());
    println!(
        "{} You can now run: {}",
        "ℹ".blue(),
        "rag ingest <path>".yellow()
    );

    Ok(())
}

/// ファイルを HTTP でダウンロードしてプログレスバー表示
async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<()> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to download {}: {}", url, e))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Download failed: HTTP {} for {}",
            response.status(),
            url
        );
    }

    // Content-Length からファイルサイズを取得
    let total_size = response.content_length();

    let pb = if let Some(size) = total_size {
        let pb = ProgressBar::new(size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "  {spinner:.cyan} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("█░"),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("  {spinner:.cyan} {bytes} downloaded")
                .unwrap(),
        );
        pb
    };

    // ストリーミングダウンロード
    let mut file = std::fs::File::create(dest)
        .map_err(|e| anyhow::anyhow!("Failed to create file {}: {}", dest.display(), e))?;

    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    use futures::StreamExt;
    use std::io::Write;

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| anyhow::anyhow!("Download error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| anyhow::anyhow!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_and_clear();

    Ok(())
}
