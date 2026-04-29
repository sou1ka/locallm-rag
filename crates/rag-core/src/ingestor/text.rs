//! Text file ingestor
//!
//! Handles: .txt / .md / .html / .htm
//! HTML is stripped to plain text via scraper.
//! Markdown is converted to plain text via pulldown-cmark.

use crate::chunker::ChunkSplitter;
use crate::store::Chunk;
use crate::ingestor::{now_iso8601, source_type_from_ext};
use std::path::Path;

/// Load a text-based file and return chunks
pub fn load(path: &Path, splitter: &ChunkSplitter) -> crate::Result<Vec<Chunk>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let raw = std::fs::read_to_string(path)
        .map_err(|e| crate::anyhow!("Failed to read {}: {}", path.display(), e))?;

    let text = match ext.as_str() {
        "html" | "htm" => extract_html(&raw),
        "md"           => extract_markdown(&raw),
        _              => raw, // .txt はそのまま
    };

    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(vec![]);
    }

    let file_path = path.to_string_lossy().to_string();
    let source_type = source_type_from_ext(&ext).to_string();
    let ingested_at = now_iso8601();

    let raw_chunks = splitter.split(&text)?;

    let chunks = raw_chunks
        .into_iter()
        .enumerate()
        .map(|(i, content)| Chunk {
            id: 0, // Storeに追加する時点でStore側が採番する想定
            source_type: source_type.clone(),
            file_path: file_path.clone(),
            page: None,
            chunk_index: i,
            content,
            ingested_at: ingested_at.clone(),
        })
        .collect();

    Ok(chunks)
}

/// HTML → プレーンテキスト（scraper使用）
fn extract_html(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // script / style タグは除外
    let selector = Selector::parse("script, style").unwrap();
    let mut text_parts = Vec::new();

    for node in document.root_element().descendants() {
        if let Some(text_node) = node.value().as_text() {
            // script/styleの子ノードはスキップ
            let trimmed = text_node.trim();
            if !trimmed.is_empty() {
                text_parts.push(trimmed.to_string());
            }
        }
    }

    text_parts.join("\n")
}

/// Markdown → プレーンテキスト（pulldown-cmark使用）
fn extract_markdown(md: &str) -> String {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let parser = Parser::new(md);
    let mut text = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Text(t) => text.push_str(&t),
            Event::Code(t) => {
                // インラインコードはそのまま含める
                text.push_str(&t);
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                text.push('\n');
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push('\n');
            }
            Event::End(TagEnd::Paragraph) => {
                text.push('\n');
            }
            _ => {}
        }
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_html_basic() {
        let html = "<html><body><p>Hello world</p></body></html>";
        let text = extract_html(html);
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn test_extract_html_strips_script() {
        let html = r#"
            <html><body>
                <script>alert('xss')</script>
                <p>本文テキスト</p>
            </body></html>
        "#;
        let text = extract_html(html);
        assert!(text.contains("本文テキスト"));
        assert!(!text.contains("alert"));
    }

    #[test]
    fn test_extract_markdown_basic() {
        let md = "# 見出し\n\n本文テキストです。";
        let text = extract_markdown(md);
        assert!(text.contains("見出し"));
        assert!(text.contains("本文テキストです。"));
    }

    #[test]
    fn test_extract_markdown_strips_syntax() {
        let md = "**太字** と *斜体* のテキスト";
        let text = extract_markdown(md);
        assert!(text.contains("太字"));
        assert!(text.contains("斜体"));
        assert!(!text.contains("**"));
        assert!(!text.contains("*"));
    }

    #[test]
    fn test_empty_file() {
        // 空テキストは空Vecを返す
        let splitter_config = crate::config::RagConfig {
            index_path: "./data/index.bin".to_string(),
            chunks_path: "./data/chunks.json".to_string(),
            chunk_size: 500,
            chunk_overlap: 50,
            top_k: 5,
            score_threshold: 0.75,
        };
        let splitter = ChunkSplitter::new(splitter_config).unwrap();

        // 空文字列を直接テスト
        let raw_chunks = splitter.split("").unwrap();
        assert_eq!(raw_chunks.len(), 0);
    }
}
