//! Video file ingestion module
//!
//! Handles video files (.mp4, .mkv, .mov, .avi)
//! Extracts metadata and subtitle content.

use std::path::Path;
use crate::store::Chunk;

/// Load and ingest a video file, returning chunks
pub fn load(path: &Path) -> crate::Result<Vec<Chunk>> {
    let text = ingest_video(path)?;

    // Video metadata is typically a single chunk
    let chunk = Chunk {
        id: 0,
        source_type: "video".to_string(),
        file_path: path.to_string_lossy().to_string(),
        page: None,
        chunk_index: 0,
        content: text,
        ingested_at: crate::ingestor::now_iso8601(),
    };

    Ok(vec![chunk])
}

/// Ingest a video file
///
/// # Arguments
/// * `path` - Path to video file
///
/// # Returns
/// Extracted metadata text from video
pub fn ingest_video(path: &Path) -> crate::Result<String> {
    // Check file exists
    if !path.exists() {
        return Err(crate::anyhow!(
            "Video file not found: {}",
            path.display()
        ));
    }

    let mut metadata = String::new();

    // Add basic file information
    metadata.push_str(&format!("Video file: {}\n", path.display()));

    // Try to extract metadata using ffprobe (if available)
    metadata.push_str("\n");
    match extract_video_metadata(path) {
        Ok(ffprobe_data) => metadata.push_str(&ffprobe_data),
        Err(_) => {
            // ffprobe not available or failed - continue with filename
            metadata.push_str("(FFprobe metadata not available)\n");
        }
    }

    // Try to load adjacent .srt subtitle file
    if let Ok(subtitle_text) = load_subtitle_file(path) {
        metadata.push_str("\n## Subtitles\n");
        metadata.push_str(&subtitle_text);
    }

    Ok(metadata.trim().to_string())
}

/// Extract video metadata using ffprobe
fn extract_video_metadata(path: &Path) -> crate::Result<String> {
    use std::process::Command;

    let path_str = path.to_string_lossy();

    // Try to run ffprobe command
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path_str.as_ref())
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(format!("FFprobe output:\n{}", stdout))
        }
        Ok(_) => Err(crate::anyhow!("ffprobe command failed")),
        Err(_) => Err(crate::anyhow!("ffprobe not found in PATH")),
    }
}

/// Load adjacent .srt subtitle file
fn load_subtitle_file(video_path: &Path) -> crate::Result<String> {
    // Look for .srt file with same name
    let mut srt_path = video_path.to_path_buf();
    srt_path.set_extension("srt");

    if srt_path.exists() {
        let content = std::fs::read_to_string(&srt_path)
            .map_err(|e| crate::anyhow!("Failed to read subtitle file: {}", e))?;

        // Parse simple SRT format
        parse_srt(&content)
    } else {
        Err(crate::anyhow!("No subtitle file found"))
    }
}

/// Parse SRT subtitle format
/// Format:
/// 1
/// 00:00:01,000 --> 00:00:04,000
/// Subtitle text here
///
/// 2
/// 00:00:05,000 --> 00:00:08,000
/// Next subtitle
fn parse_srt(content: &str) -> crate::Result<String> {
    let mut subtitles = String::new();
    let mut lines = content.lines().peekable();

    while lines.peek().is_some() {
        // Skip sequence number
        if let Some(line) = lines.next() {
            if line.trim().chars().all(|c| c.is_numeric()) {
                // Found sequence number, skip it
                lines.next(); // Skip timecode line
                
                // Collect subtitle text until empty line
                while let Some(text_line) = lines.next() {
                    if text_line.trim().is_empty() {
                        break;
                    }
                    subtitles.push_str(text_line);
                    subtitles.push(' ');
                }
                subtitles.push('\n');
            }
        }
    }

    Ok(subtitles.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_video_nonexistent() {
        let result = ingest_video(Path::new("./nonexistent.mp4"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_srt() {
        let srt_content = r#"1
00:00:01,000 --> 00:00:04,000
Hello world

2
00:00:05,000 --> 00:00:08,000
This is a test
"#;
        let result = parse_srt(srt_content).unwrap();
        assert!(result.contains("Hello world"));
        assert!(result.contains("This is a test"));
    }

    #[test]
    fn test_parse_srt_empty() {
        let result = parse_srt("").unwrap();
        assert_eq!(result, "");
    }
}
