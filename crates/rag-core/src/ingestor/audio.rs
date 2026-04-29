//! Audio file ingestion module
//!
//! Handles audio files (.mp3, .flac, .m4a, .ogg, .wav)
//! Extracts metadata tags for ingestion.

use std::path::Path;
use crate::store::Chunk;

/// Load and ingest an audio file, returning chunks
///
/// Audio files are converted to metadata text without splitting (single chunk per file)
pub fn load(path: &Path) -> crate::Result<Vec<Chunk>> {
    let text = ingest_audio(path)?;

    // Audio is typically converted to a single chunk
    let chunk = Chunk {
        id: 0,
        source_type: "audio".to_string(),
        file_path: path.to_string_lossy().to_string(),
        page: None,
        chunk_index: 0,
        content: text,
        ingested_at: crate::ingestor::now_iso8601(),
    };

    Ok(vec![chunk])
}

/// Ingest an audio file
///
/// # Arguments
/// * `path` - Path to audio file
///
/// # Returns
/// Extracted metadata text from audio
pub fn ingest_audio(path: &Path) -> crate::Result<String> {
    // Check file exists
    if !path.exists() {
        return Err(crate::anyhow!(
            "Audio file not found: {}",
            path.display()
        ));
    }

    // Check file extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "mp3" => extract_mp3_metadata(path),
        "m4a" => extract_m4a_metadata(path),
        "flac" | "ogg" | "wav" => {
            // FLAC / OGG metadata extraction would require additional dependencies
            // For now, return a placeholder
            Ok(format!(
                "Audio file: {} (metadata extraction not yet implemented for {})",
                path.display(),
                ext
            ))
        }
        _ => Err(crate::anyhow!("Unsupported audio format: {}", ext)),
    }
}

/// Extract MP3 metadata using ID3 tags
fn extract_mp3_metadata(path: &Path) -> crate::Result<String> {
    use id3::{Tag, TagLike};  // TagLike を追加

    let tag = Tag::read_from_path(path)
        .map_err(|e| crate::anyhow!("Failed to read ID3 tag: {}", e))?;

    let mut parts = Vec::new();

    // title() は Option<&str> を返す。.next() は不要
    if let Some(v) = tag.title()  { parts.push(format!("タイトル: {}", v)); }
    if let Some(v) = tag.artist() { parts.push(format!("アーティスト: {}", v)); }
    if let Some(v) = tag.album()  { parts.push(format!("アルバム: {}", v)); }
    if let Some(v) = tag.year()   { parts.push(format!("年: {}", v)); }
    if let Some(v) = tag.genre()  { parts.push(format!("ジャンル: {}", v)); }

    // comments() は Iterator<Item = &Comment> を返す。Option ではない
    for comment in tag.comments() {
        if !comment.text.is_empty() {
            parts.push(format!("コメント: {}", comment.text));
        }
    }

    if parts.is_empty() {
        parts.push(format!(
            "Audio file: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
    }

    Ok(parts.join("\n"))
}

/// Extract M4A metadata using mp4ameta
fn extract_m4a_metadata(path: &Path) -> crate::Result<String> {
    use mp4ameta::Tag;

    let tag = Tag::read_from_path(path)
        .map_err(|e| crate::anyhow!("Failed to read M4A tag: {}", e))?;

    let mut parts = Vec::new();

    // mp4ameta の title() 等も Option<&str> を返す。.next() は不要
    if let Some(v) = tag.title()  { parts.push(format!("タイトル: {}", v)); }
    if let Some(v) = tag.artist() { parts.push(format!("アーティスト: {}", v)); }
    if let Some(v) = tag.album()  { parts.push(format!("アルバム: {}", v)); }
    if let Some(v) = tag.year()   { parts.push(format!("年: {}", v)); }
    if let Some(v) = tag.genre()  { parts.push(format!("ジャンル: {}", v)); }

    // comments() は Iterator<Item = &str>。Option ではない
    for comment in tag.comments() {
        if !comment.is_empty() {
            parts.push(format!("コメント: {}", comment));
        }
    }

    if parts.is_empty() {
        parts.push(format!(
            "Audio file: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
    }

    Ok(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_audio_nonexistent() {
        let result = ingest_audio(Path::new("./nonexistent.mp3"));
        assert!(result.is_err());
    }

    #[test]
    fn test_ingest_audio_unsupported_format() {
        let result = ingest_audio(Path::new("./test.unknown"));
        assert!(result.is_err());
    }
}
