//! SQLite SQL dump file ingestion module
//!
//! Handles SQL dump files (.sql) containing INSERT statements.
//! Parses and extracts data from INSERT statements for ingestion.

use std::path::Path;
use crate::chunker::ChunkSplitter;
use crate::store::Chunk;

/// Load and ingest a SQL dump file, returning chunks
pub fn load(path: &Path, splitter: &ChunkSplitter) -> crate::Result<Vec<Chunk>> {
    let text = ingest_sql_dump(path)?;

    // Split into chunks
    let chunk_texts = splitter.split(&text)?;

    // Convert to Chunk structs
    let chunks = chunk_texts
        .into_iter()
        .enumerate()
        .map(|(idx, content)| Chunk {
            id: idx,
            source_type: "sqlite_dump".to_string(),
            file_path: path.to_string_lossy().to_string(),
            page: None,
            chunk_index: idx,
            content,
            ingested_at: crate::ingestor::now_iso8601(),
        })
        .collect();

    Ok(chunks)
}

/// Ingest a SQL dump file
///
/// # Arguments
/// * `path` - Path to SQL dump file (.sql)
///
/// # Returns
/// Text representation of INSERT statements
pub fn ingest_sql_dump(path: &Path) -> crate::Result<String> {
    // Check file exists
    if !path.exists() {
        return Err(crate::anyhow!(
            "SQL dump file not found: {}",
            path.display()
        ));
    }

    // Read SQL file
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::anyhow!("Failed to read SQL dump file: {}", e))?;

    // Parse and extract INSERT statements
    let mut output = String::new();
    let mut current_table = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with("/*") {
            continue;
        }

        // Extract table name from CREATE TABLE statements
        if trimmed.to_uppercase().starts_with("CREATE TABLE") {
            if let Some(table_name) = extract_table_name(trimmed) {
                current_table = table_name;
                output.push_str(&format!("## Table: {}\n", current_table));
            }
            continue;
        }

        // Extract data from INSERT statements
        if trimmed.to_uppercase().starts_with("INSERT INTO") {
            if let Some(values_str) = extract_insert_values(trimmed) {
                output.push_str(&values_str);
                output.push('\n');
            }
        }
    }

    Ok(output.trim().to_string())
}

/// Extract table name from CREATE TABLE statement
fn extract_table_name(line: &str) -> Option<String> {
    // Simple parser: "CREATE TABLE tablename (...)"
    let upper = line.to_uppercase();
    if let Some(pos) = upper.find("CREATE TABLE") {
        let after_create = &line[pos + 12..].trim_start();
        // Take tokens until we hit '(' or whitespace
        let table_name = after_create
            .split(|c: char| c.is_whitespace() || c == '(')
            .next()
            .unwrap_or("")
            .trim_matches('`')
            .trim_matches('"');

        if !table_name.is_empty() {
            return Some(table_name.to_string());
        }
    }
    None
}

/// Extract VALUES part from INSERT statement
fn extract_insert_values(line: &str) -> Option<String> {
    // Find VALUES keyword
    let upper = line.to_uppercase();
    if let Some(pos) = upper.find("VALUES") {
        let after_values = &line[pos + 6..].trim_start();
        // Extract the parenthesized content
        if let Some(start) = after_values.find('(') {
            if let Some(end) = after_values.rfind(')') {
                if end > start {
                    let values = &after_values[start + 1..end];
                    // Replace NULL and quoted strings for readability
                    let cleaned = values
                        .replace("NULL", "[NULL]")
                        .replace("\\'", "'");
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_table_name() {
        let line = "CREATE TABLE users (id INTEGER, name TEXT)";
        let result = extract_table_name(line);
        assert_eq!(result, Some("users".to_string()));
    }

    #[test]
    fn test_extract_table_name_quoted() {
        let line = "CREATE TABLE `users` (id INTEGER)";
        let result = extract_table_name(line);
        assert_eq!(result, Some("users".to_string()));
    }

    #[test]
    fn test_extract_insert_values() {
        let line = "INSERT INTO users VALUES (1, 'John', 'john@example.com')";
        let result = extract_insert_values(line);
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val.contains("John"));
    }

    #[test]
    fn test_ingest_sql_dump_nonexistent() {
        let result = ingest_sql_dump(Path::new("./nonexistent.sql"));
        assert!(result.is_err());
    }
}
