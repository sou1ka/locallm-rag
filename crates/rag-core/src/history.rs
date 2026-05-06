//! Conversation history persistence
//!
//! Saves and loads conversations as JSON files under history_dir.
//! Filename format: {YYYYMMDD_HHMMSS}_{title}.json

use crate::conversation::Conversation;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// History manager for conversation persistence
pub struct HistoryManager {
    history_dir: PathBuf,
}

impl HistoryManager {
    pub fn new(history_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = history_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| anyhow::anyhow!("Failed to create history dir: {}", e))?;
        Ok(Self { history_dir: dir })
    }

    /// Save a conversation to JSON file
    pub fn save(&self, conversation: &Conversation) -> Result<PathBuf> {
        let filename = make_filename(conversation);
        let path = self.history_dir.join(&filename);

        let json = serde_json::to_string_pretty(conversation)
            .map_err(|e| anyhow::anyhow!("Failed to serialize conversation: {}", e))?;

        std::fs::write(&path, json)
            .map_err(|e| anyhow::anyhow!("Failed to write history file: {}", e))?;

        Ok(path)
    }

    /// Load a conversation from JSON file by path
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Conversation> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to read history file: {}", e))?;

        let conversation: Conversation = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse history file: {}", e))?;

        Ok(conversation)
    }

    /// List all conversation files (newest first)
    pub fn list(&self) -> Result<Vec<HistoryEntry>> {
        let mut entries = Vec::new();

        for entry in std::fs::read_dir(&self.history_dir)
            .map_err(|e| anyhow::anyhow!("Failed to read history dir: {}", e))?
        {
            let entry = entry
                .map_err(|e| anyhow::anyhow!("Failed to read dir entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            if let Some(info) = parse_filename(&path) {
                entries.push(info);
            }
        }

        // 新しい順にソート（ファイル名が日時なのでそのまま逆順）
        entries.sort_by(|a, b| b.datetime.cmp(&a.datetime));

        Ok(entries)
    }

    /// Load all conversations (newest first)
    pub fn load_all(&self) -> Result<Vec<Conversation>> {
        let entries = self.list()?;
        let mut conversations = Vec::new();

        for entry in entries {
            match self.load(&entry.path) {
                Ok(conv) => conversations.push(conv),
                Err(e) => eprintln!("[WARN] Failed to load {}: {}", entry.path.display(), e),
            }
        }

        Ok(conversations)
    }

    /// Delete a conversation file by session_id
    pub fn delete(&self, session_id: &str) -> Result<bool> {
        let entries = self.list()?;

        for entry in entries {
            if entry.session_id == session_id {
                std::fs::remove_file(&entry.path)
                    .map_err(|e| anyhow::anyhow!("Failed to delete: {}", e))?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Find a conversation file path by session_id
    pub fn find_path(&self, session_id: &str) -> Option<PathBuf> {
        let entries = self.list().ok()?;
        entries
            .into_iter()
            .find(|e| e.session_id == session_id)
            .map(|e| e.path)
    }
}

/// Metadata parsed from history filename
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub path: PathBuf,
    pub session_id: String,   // YYYYMMDD_HHMMSS
    pub title: String,
    pub datetime: String,     // YYYYMMDD_HHMMSS（ソート用）
}

/// Generate filename from conversation
/// Format: {YYYYMMDD_HHMMSS}_{sanitized_title}.json
fn make_filename(conversation: &Conversation) -> String {
    // session_id が既に YYYYMMDD_HHMMSS 形式であればそのまま使う
    // そうでなければ現在時刻を使う
    let datetime = if looks_like_datetime(&conversation.id) {
        conversation.id.clone()
    } else {
        current_datetime()
    };

    let title = conversation
        .title
        .as_deref()
        .unwrap_or("untitled");

    let sanitized = sanitize_filename(title);

    format!("{}_{}.json", datetime, sanitized)
}

/// Parse filename to extract metadata
/// Expected: {YYYYMMDD_HHMMSS}_{title}.json
fn parse_filename(path: &Path) -> Option<HistoryEntry> {
    let stem = path.file_stem()?.to_str()?;

    // 最初の17文字が YYYYMMDD_HHMMSS 形式かチェック
    if stem.len() < 17 {
        return None;
    }

    let datetime = &stem[..15]; // YYYYMMDD_HHMMSS = 15文字
    let title = if stem.len() > 16 {
        stem[16..].to_string() // _ の次から
    } else {
        "untitled".to_string()
    };

    Some(HistoryEntry {
        path: path.to_path_buf(),
        session_id: datetime.to_string(),
        title,
        datetime: datetime.to_string(),
    })
}

/// Check if string looks like YYYYMMDD_HHMMSS
fn looks_like_datetime(s: &str) -> bool {
    if s.len() < 15 {
        return false;
    }
    s.chars().take(8).all(|c| c.is_ascii_digit())
        && s.chars().nth(8) == Some('_')
        && s.chars().skip(9).take(6).all(|c| c.is_ascii_digit())
}

/// Get current datetime as YYYYMMDD_HHMMSS
pub fn current_datetime() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // chrono非依存のシンプル実装
    // 秒からYYYYMMDD_HHMMSSに変換
    let secs = secs as i64;
    let (year, month, day, hour, min, sec) = unix_to_datetime(secs);

    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        year, month, day, hour, min, sec
    )
}

/// Unix timestamp to (year, month, day, hour, min, sec)
fn unix_to_datetime(secs: i64) -> (i64, i64, i64, i64, i64, i64) {
    let sec = secs % 60;
    let min = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;

    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days);

    (year, month, day, hour, min, sec)
}

/// Days since epoch to (year, month, day)
fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let mut y = 1970i64;
    let mut d = days;

    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }

    let month_days: Vec<i64> = if is_leap(y) {
        vec![31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        vec![31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 1i64;
    for md in &month_days {
        if d < *md {
            break;
        }
        d -= md;
        m += 1;
    }

    (y, m, d + 1)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Sanitize title for use in filename
fn sanitize_filename(title: &str) -> String {
    title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .take(50) // 長すぎるタイトルは切り詰め
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello/world"), "hello_world");
        assert_eq!(sanitize_filename("test:file"), "test_file");
        assert_eq!(sanitize_filename("normal title"), "normal title");
    }

    #[test]
    fn test_looks_like_datetime() {
        assert!(looks_like_datetime("20260506_143022"));
        assert!(!looks_like_datetime("abc"));
        assert!(!looks_like_datetime("2026050"));
    }

    #[test]
    fn test_current_datetime_format() {
        let dt = current_datetime();
        assert_eq!(dt.len(), 15);
        assert_eq!(dt.chars().nth(8), Some('_'));
    }

    #[test]
    fn test_make_filename_untitled() {
        let conv = Conversation::new("20260506_143022".to_string());
        let filename = make_filename(&conv);
        assert!(filename.ends_with(".json"));
        assert!(filename.contains("untitled"));
    }
}
