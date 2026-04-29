//! SQLite database ingestion module
//!
//! Handles SQLite database files (.db, .sqlite, .sqlite3)
//! Extracts table structures and content for ingestion.

use std::path::Path;
use crate::chunker::ChunkSplitter;
use crate::store::Chunk;

/// Load and ingest a SQLite database file, returning chunks
pub fn load(path: &Path, splitter: &ChunkSplitter) -> crate::Result<Vec<Chunk>> {
    let text = ingest_sqlite(path)?;

    // Split into chunks
    let chunk_texts = splitter.split(&text)?;

    // Convert to Chunk structs
    let chunks = chunk_texts
        .into_iter()
        .enumerate()
        .map(|(idx, content)| Chunk {
            id: idx,
            source_type: "sqlite".to_string(),
            file_path: path.to_string_lossy().to_string(),
            page: None,
            chunk_index: idx,
            content,
            ingested_at: crate::ingestor::now_iso8601(),
        })
        .collect();

    Ok(chunks)
}

/// Ingest a SQLite database file
///
/// # Arguments
/// * `path` - Path to SQLite database file
///
/// # Returns
/// Text representation of database structure and content
pub fn ingest_sqlite(path: &Path) -> crate::Result<String> {
    use rusqlite::Connection;

    // Check file exists
    if !path.exists() {
        return Err(crate::anyhow!(
            "SQLite database file not found: {}",
            path.display()
        ));
    }

    // Open database connection
    let conn = Connection::open(path)
        .map_err(|e| crate::anyhow!("Failed to open SQLite database: {}", e))?;

    let mut output = String::new();

    // Get all table names
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .map_err(|e| crate::anyhow!("Failed to query tables: {}", e))?;

    let tables = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| crate::anyhow!("Failed to read table names: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| crate::anyhow!("Failed to collect table names: {}", e))?;

    // Process each table
    for table_name in tables {
        output.push_str(&format!("## Table: {}\n", table_name));

        // Get table schema
        let mut schema_stmt = conn
            .prepare(&format!("PRAGMA table_info({})", table_name))
            .map_err(|e| crate::anyhow!("Failed to get table schema: {}", e))?;

        let columns: Vec<String> = schema_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| crate::anyhow!("Failed to read column names: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::anyhow!("Failed to collect columns: {}", e))?;

        if !columns.is_empty() {
            output.push_str(&format!("Columns: {}\n", columns.join(" | ")));
        }

        // Get table data
        let mut data_stmt = conn
            .prepare(&format!("SELECT * FROM {}", table_name))
            .map_err(|e| crate::anyhow!("Failed to query table: {}", e))?;

        let rows = data_stmt
            .query_map([], |row| {
                let mut values = Vec::new();
                for i in 0..columns.len() {
                    if let Ok(val) = row.get::<_, String>(i) {
                        values.push(val);
                    } else if let Ok(val) = row.get::<_, i64>(i) {
                        values.push(val.to_string());
                    } else {
                        values.push("NULL".to_string());
                    }
                }
                Ok(values.join(" | "))
            })
            .map_err(|e| crate::anyhow!("Failed to read table rows: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::anyhow!("Failed to collect rows: {}", e))?;

        for row_text in rows {
            output.push_str(&row_text);
            output.push('\n');
        }

        output.push('\n');
    }

    Ok(output.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_sqlite_nonexistent() {
        let result = ingest_sqlite(Path::new("./nonexistent.db"));
        assert!(result.is_err());
    }
}
