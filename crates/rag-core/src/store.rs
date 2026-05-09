//! Vector store backed by SQLite
//!
//! chunks テーブル: メタデータ + 本文（SQL で直接参照可能）
//! embeddings テーブル: f32 ベクトルを BLOB 保存（バイナリ効率）

use crate::config::RagConfig;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// チャンクメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: usize,
    pub source_type: String,
    pub file_path: String,
    pub page: Option<i32>,
    pub chunk_index: usize,
    pub content: String,
    pub ingested_at: String,
}

/// SQLite バックの Vector Store
pub struct Store {
    conn: Connection,
    chunks: Vec<Chunk>,
    embeddings: Vec<Vec<f32>>,
    config: RagConfig,
}

impl Store {
    /// DB を開く（存在しなければ作成）
    pub fn open(config: RagConfig, db_path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = db_path.as_ref();
        if path.to_str() != Some(":memory:") {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| crate::anyhow!("Failed to create DB dir: {}", e))?;
                }
            }
        }

        let conn = Connection::open(path)
            .map_err(|e| crate::anyhow!("Failed to open DB '{}': {}", path.display(), e))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS chunks (
                 id          INTEGER PRIMARY KEY,
                 source_type TEXT NOT NULL,
                 file_path   TEXT NOT NULL,
                 page        INTEGER,
                 chunk_index INTEGER NOT NULL,
                 content     TEXT NOT NULL,
                 ingested_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS embeddings (
                 chunk_id INTEGER PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
                 vector   BLOB NOT NULL
             );",
        )
        .map_err(|e| crate::anyhow!("Failed to init DB schema: {}", e))?;

        let (chunks, embeddings) = Self::load_all(&conn)?;
        Ok(Self { conn, chunks, embeddings, config })
    }

    /// チャンク + 埋め込みを追加し DB に即時書き込む
    pub fn add_chunk(&mut self, embedding: Vec<f32>, chunk: Chunk) -> crate::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO chunks
             (id, source_type, file_path, page, chunk_index, content, ingested_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                chunk.id as i64,
                chunk.source_type,
                chunk.file_path,
                chunk.page,
                chunk.chunk_index as i64,
                chunk.content,
                chunk.ingested_at
            ],
        )
        .map_err(|e| crate::anyhow!("Failed to insert chunk: {}", e))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO embeddings (chunk_id, vector) VALUES (?1, ?2)",
            params![chunk.id as i64, f32_to_bytes(&embedding)],
        )
        .map_err(|e| crate::anyhow!("Failed to insert embedding: {}", e))?;

        self.embeddings.push(embedding);
        self.chunks.push(chunk);
        Ok(())
    }

    /// 互換性維持のための no-op（SQLite は add_chunk で即時永続化）
    pub fn save(&self) -> crate::Result<()> {
        Ok(())
    }

    /// 全データを削除
    pub fn clear(&mut self) -> crate::Result<()> {
        // ON DELETE CASCADE により embeddings も連動削除される
        self.conn
            .execute_batch("PRAGMA foreign_keys=ON; DELETE FROM chunks;")
            .map_err(|e| crate::anyhow!("Failed to clear DB: {}", e))?;
        self.chunks.clear();
        self.embeddings.clear();
        Ok(())
    }

    /// 指定パスプレフィックスに一致するチャンクを削除し、削除件数を返す
    pub fn delete_by_path(&mut self, path_prefix: &str) -> crate::Result<usize> {
        let pattern = format!("{}%", path_prefix);
        let removed = self.conn
            .execute(
                "PRAGMA foreign_keys=ON; DELETE FROM chunks WHERE file_path LIKE ?1",
                params![pattern],
            )
            .map_err(|e| crate::anyhow!("Failed to delete chunks: {}", e))?;
        let (chunks, embeddings) = Self::load_all(&self.conn)?;
        self.chunks = chunks;
        self.embeddings = embeddings;
        Ok(removed)
    }

    /// コサイン類似度検索
    pub fn search(&self, embedding: &[f32], k: Option<usize>) -> crate::Result<Vec<(usize, f32)>> {
        let k = k.unwrap_or(self.config.top_k);
        let mut results: Vec<(usize, f32)> = self
            .embeddings
            .iter()
            .enumerate()
            .map(|(i, e)| (i, cosine_similarity(embedding, e)))
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results.into_iter().take(k).collect())
    }

    pub fn get_chunk(&self, id: usize) -> Option<&Chunk> { self.chunks.get(id) }
    pub fn chunks(&self) -> &[Chunk] { &self.chunks }
    pub fn len(&self) -> usize { self.chunks.len() }
    pub fn is_empty(&self) -> bool { self.chunks.is_empty() }

    // ── 内部ヘルパー ─────────────────────────────────────────────────────────

    fn load_all(conn: &Connection) -> crate::Result<(Vec<Chunk>, Vec<Vec<f32>>)> {
        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.source_type, c.file_path, c.page,
                        c.chunk_index, c.content, c.ingested_at, e.vector
                 FROM chunks c JOIN embeddings e ON c.id = e.chunk_id
                 ORDER BY c.id",
            )
            .map_err(|e| crate::anyhow!("DB prepare failed: {}", e))?;

        let mut chunks = Vec::new();
        let mut embeddings = Vec::new();

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    Chunk {
                        id:          row.get::<_, i64>(0)? as usize,
                        source_type: row.get(1)?,
                        file_path:   row.get(2)?,
                        page:        row.get(3)?,
                        chunk_index: row.get::<_, i64>(4)? as usize,
                        content:     row.get(5)?,
                        ingested_at: row.get(6)?,
                    },
                    row.get::<_, Vec<u8>>(7)?,
                ))
            })
            .map_err(|e| crate::anyhow!("DB query failed: {}", e))?;

        for row in rows {
            let (chunk, blob) = row.map_err(|e| crate::anyhow!("DB row error: {}", e))?;
            embeddings.push(bytes_to_f32(&blob));
            chunks.push(chunk);
        }

        Ok((chunks, embeddings))
    }
}

fn f32_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RagConfig {
        RagConfig {
            db_path: ":memory:".to_string(),
            chunk_size: 512,
            chunk_overlap: 128,
            top_k: 5,
            score_threshold: 0.5,
        }
    }

    fn test_chunk(id: usize) -> Chunk {
        Chunk {
            id,
            source_type: "text".to_string(),
            file_path: "./test.txt".to_string(),
            page: None,
            chunk_index: 0,
            content: format!("Test chunk {}", id),
            ingested_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_open_and_empty() {
        let store = Store::open(test_config(), ":memory:").unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_add_and_search() {
        let mut store = Store::open(test_config(), ":memory:").unwrap();
        let emb = vec![0.1f32; 8];
        store.add_chunk(emb.clone(), test_chunk(0)).unwrap();
        assert_eq!(store.len(), 1);
        let results = store.search(&emb, Some(1)).unwrap();
        assert_eq!(results.len(), 1);
        assert!((results[0].1 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_clear() {
        let mut store = Store::open(test_config(), ":memory:").unwrap();
        store.add_chunk(vec![0.1f32; 8], test_chunk(0)).unwrap();
        store.clear().unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
    }
}
