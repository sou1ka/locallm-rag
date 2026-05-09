//! Spreadsheet ingestion module
//!
//! Handles CSV and Excel (.xlsx, .xls, .ods) files.
//! Converts each row to text format for ingestion.

use std::path::Path;
use crate::chunker::ChunkSplitter;
use crate::store::Chunk;

/// Load and ingest a spreadsheet file, returning chunks
pub fn load(path: &Path, splitter: &ChunkSplitter) -> crate::Result<Vec<Chunk>> {
    // Get file extension to determine format
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Extract text from the appropriate format
    let text = match ext.as_str() {
        "csv" => ingest_csv(path)?,
        "xlsx" | "xls" => ingest_excel(path)?,
        "ods" => return Err(crate::anyhow!("ODS format not yet supported")),
        _ => return Err(crate::anyhow!("Unknown spreadsheet format: {}", ext)),
    };

    // Split into chunks
    let chunk_texts = splitter.split(&text)?;

    // Convert to Chunk structs
    let chunks = chunk_texts
        .into_iter()
        .enumerate()
        .map(|(idx, content)| Chunk {
            id: idx,
            source_type: "spreadsheet".to_string(),
            file_path: path.to_string_lossy().to_string(),
            page: None,
            chunk_index: idx,
            content,
            ingested_at: crate::ingestor::now_iso8601(),
        })
        .collect();

    Ok(chunks)
}

/// Ingest a CSV file
///
/// UTF-8（BOM付き含む）と ShiftJIS（CP932）を自動判別して読み込む。
pub fn ingest_csv(path: &Path) -> crate::Result<String> {
    let bytes = std::fs::read(path)
        .map_err(|e| crate::anyhow!("Failed to read CSV file {}: {}", path.display(), e))?;

    let content = crate::ingestor::decode_to_utf8(&bytes);

    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let mut output = String::new();

    for result in reader.records() {
        let record = result
            .map_err(|e| crate::anyhow!("Failed to read CSV record: {}", e))?;
        let row_text = record.iter().collect::<Vec<_>>().join(" | ");
        output.push_str(&row_text);
        output.push('\n');
    }

    Ok(output.trim().to_string())
}


/// Ingest an Excel file (.xlsx, .xls, .ods)
///
/// # Arguments
/// * `path` - Path to Excel file
///
/// # Returns
/// Text representation of all sheets and rows
pub fn ingest_excel(path: &Path) -> crate::Result<String> {
    // Check file exists
    if !path.exists() {
        return Err(crate::anyhow!(
            "Excel file not found: {}",
            path.display()
        ));
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "xlsx" | "xls" => ingest_excel_calamine(path),
        "ods" => Err(crate::anyhow!("ODS format not yet supported")),
        _ => Err(crate::anyhow!("Unknown spreadsheet format: {}", extension)),
    }
}

/// Ingest Excel file using calamine
fn ingest_excel_calamine(path: &Path) -> crate::Result<String> {
    use calamine::{open_workbook, Reader, Xlsx};

    let mut workbook: Xlsx<_> = open_workbook(path)
        .map_err(|e| crate::anyhow!("Failed to open Excel file: {}", e))?;

    let mut output = String::new();

    // Iterate through all sheets
    let sheet_names = workbook.sheet_names().to_vec();

    for sheet_name in sheet_names {
        // Add sheet name as header
        output.push_str(&format!("## Sheet: {}\n", sheet_name));

        // Read sheet
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            for row in range.rows() {
                let row_text = row
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ");
                output.push_str(&row_text);
                output.push('\n');
            }
        }

        output.push('\n');
    }

    Ok(output.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_csv_nonexistent() {
        let result = ingest_csv(Path::new("./nonexistent.csv"));
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_utf8() {
        let input = "名前,年齢\n山田,30\n".as_bytes().to_vec();
        let result = crate::ingestor::decode_to_utf8(&input);
        assert!(result.contains("名前"));
    }

    #[test]
    fn test_decode_utf8_bom() {
        let mut input = b"\xEF\xBB\xBF".to_vec();
        input.extend_from_slice("名前,年齢\n".as_bytes());
        let result = crate::ingestor::decode_to_utf8(&input);
        // BOM が除去されて正しくデコードされる
        assert!(result.starts_with("名前"));
    }

    #[test]
    fn test_decode_shiftjis() {
        // "名前" の ShiftJIS バイト列
        let shiftjis: Vec<u8> = vec![0x96, 0xBC, 0x91, 0x4F];
        let result = crate::ingestor::decode_to_utf8(&shiftjis);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_ingest_excel_nonexistent() {
        let result = ingest_excel(Path::new("./nonexistent.xlsx"));
        assert!(result.is_err());
    }

    #[test]
    fn test_ingest_excel_unknown_format() {
        // Create a temporary file with unknown extension
        let result = ingest_excel(Path::new("./test.unknown"));
        assert!(result.is_err());
    }

    #[test]
    fn test_ingest_excel_ods_unsupported() {
        let result = ingest_excel(Path::new("./test.ods"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ODS"));
    }
}
