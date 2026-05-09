//! Ingestor module
//!
//! Recursively walks directories and dispatches to format-specific loaders.
//! Returns Vec<Chunk> for storage in the vector store.

pub mod text;
pub mod pdf;
pub mod sqlite;
pub mod sqlite_dump;
pub mod audio;
pub mod image;
pub mod video;
pub mod spreadsheet;

use crate::chunker::ChunkSplitter;
use crate::config::RagConfig;
use crate::store::Chunk;
use std::path::Path;
use walkdir::WalkDir;

/// Ingest all files under a directory path recursively
pub fn ingest_directory(
    path: &Path,
    extensions: Option<&[String]>,
    config: &RagConfig,
) -> crate::Result<Vec<Chunk>> {
    let mut all_chunks = Vec::new();
    let splitter = ChunkSplitter::new(config.clone())?;

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();

        // 拡張子フィルタ
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(exts) = extensions {
            if !exts.iter().any(|e| e.to_lowercase() == ext) {
                continue;
            }
        }

        // 拡張子でディスパッチ
        match dispatch(file_path, &ext, &splitter) {
            Ok(chunks) => all_chunks.extend(chunks),
            Err(e) => {
                // ファイル単位のエラーはスキップしてログだけ出す
                eprintln!("[WARN] skipped {}: {}", file_path.display(), e);
            }
        }
    }

    Ok(all_chunks)
}

/// 単一ファイルをパスと種別で直接インジェスト
pub fn ingest_file(path: &Path, config: &RagConfig) -> crate::Result<Vec<Chunk>> {
    let splitter = ChunkSplitter::new(config.clone())?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    dispatch(path, &ext, &splitter)
}

/// 拡張子 → 各モジュールへのディスパッチ
fn dispatch(
    path: &Path,
    ext: &str,
    splitter: &ChunkSplitter,
) -> crate::Result<Vec<Chunk>> {
    match ext {
        // テキスト系
        "txt" | "md" | "html" | "htm" => text::load(path, splitter),

        // PDF（テキスト/スキャン自動判定はpdf.rs側で行う）
        "pdf" => pdf::load(path, splitter),

        // スプレッドシート
        "csv" | "xlsx" | "ods" => spreadsheet::load(path, splitter),

        // SQLite
        "db" | "sqlite" | "sqlite3" => sqlite::load(path, splitter),

        // SQLite dump
        "sql" => sqlite_dump::load(path, splitter),

        // 音声
        "mp3" | "flac" | "m4a" | "ogg" | "wav" => audio::load(path),

        // 画像
        "jpg" | "jpeg" | "png" | "tiff" | "tif" | "webp" => image::load(path),

        // 動画
        "mp4" | "mkv" | "mov" | "avi" => video::load(path),

        // 非対応
        _ => {
            Ok(vec![]) // サイレントスキップ
        }
    }
}

/// ファイルパスからソース種別文字列を生成
pub fn source_type_from_ext(ext: &str) -> &'static str {
    match ext {
        "txt" | "md" | "html" | "htm" => "text",
        "pdf" => "pdf",
        "csv" | "xlsx" | "ods" => "spreadsheet",
        "db" | "sqlite" | "sqlite3" => "sqlite",
        "sql" => "sqlite_dump",
        "mp3" | "flac" | "m4a" | "ogg" | "wav" => "audio",
        "jpg" | "jpeg" | "png" | "tiff" | "tif" | "webp" => "image",
        "mp4" | "mkv" | "mov" | "avi" => "video",
        _ => "unknown",
    }
}

/// 現在時刻をUnixタイムスタンプ文字列で返す
pub fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

/// ファイルバイト列を UTF-8 文字列にデコード（ShiftJIS フォールバック対応）
pub fn decode_to_utf8(bytes: &[u8]) -> String {
    let stripped = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if std::str::from_utf8(stripped).is_ok() {
        return String::from_utf8_lossy(stripped).into_owned();
    }
    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
    decoded.into_owned()
}
