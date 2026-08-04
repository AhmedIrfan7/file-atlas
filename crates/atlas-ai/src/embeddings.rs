//! Local embedding index and semantic search over file names.
//!
//! Deliberately scoped to file names (plus category and containing folder
//! for a little extra context), not extracted file contents: reading and
//! embedding the text inside every PDF/DOCX is a real feature with its own
//! failure modes (encoding, scanned-image PDFs with no text layer, huge
//! files) and is deferred rather than half-built here. See ADR 0011.
//!
//! Similarity search is a linear scan over stored vectors, computed in Rust
//! rather than a SQLite vector extension: at the scale this targets (tens
//! of thousands of files), a full scan is fast enough, and it avoids a new
//! runtime dependency for approximate nearest-neighbor indexing before
//! anyone has hit a real performance ceiling.

use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::provider::EmbeddingProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbedProgress {
    pub files_embedded: u64,
    pub files_total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedStats {
    pub files_embedded: u64,
    pub errors: u64,
}

/// One semantic search result, with enough display detail (name, size,
/// category, last-modified) that the UI never needs a second query per hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarFile {
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
    pub category: Option<String>,
    pub score: f32,
}

/// Embed every live file that has no embedding yet under `provider`'s model.
///
/// Switching embedding models naturally means every file is "pending" again,
/// since the model name is part of what makes a row current.
pub fn build_embedding_index(
    conn: &Connection,
    provider: &dyn EmbeddingProvider,
    now: i64,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(EmbedProgress),
) -> rusqlite::Result<EmbedStats> {
    let model = provider.model_name();
    let candidates = pending_files(conn, model)?;
    let files_total = candidates.len() as u64;
    let mut files_embedded = 0u64;
    let mut errors = 0u64;

    for (path, name, category, parent) in candidates {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let text = embedding_text(&name, category.as_deref(), &parent);
        match provider.embed(&text) {
            Ok(vector) => {
                store_embedding(conn, &path, model, &vector, now)?;
                files_embedded += 1;
            }
            Err(err) => {
                tracing::warn!(path = %path, error = %err, "embedding failed");
                errors += 1;
            }
        }
        on_progress(EmbedProgress {
            files_embedded: files_embedded + errors,
            files_total,
        });
    }

    Ok(EmbedStats {
        files_embedded,
        errors,
    })
}

/// Embed `query_text` and rank every stored embedding by cosine similarity,
/// highest first.
///
/// A convenience wrapper around `rank_by_similarity` for callers (tests,
/// simple scripts) that do not care about how long a database connection
/// stays locked. `apps/desktop`'s Tauri command calls `rank_by_similarity`
/// directly instead, embedding the query *before* taking the connection
/// lock: this function's own embed-then-query shape would otherwise hold
/// the app's single shared connection locked for the duration of a network
/// call to the embedding provider, which is exactly the kind of thing ADR
/// 0003's single-writer model is supposed to only accept for background
/// jobs that say so on purpose, not for an ordinary read command.
pub fn semantic_search(
    conn: &Connection,
    provider: &dyn EmbeddingProvider,
    query_text: &str,
    limit: u32,
) -> crate::provider::Result<Vec<SimilarFile>> {
    let query_vector = provider.embed(query_text)?;
    rank_by_similarity(conn, &query_vector, provider.model_name(), limit)
        .map_err(|e| crate::provider::AiError::Request(e.to_string()))
}

/// Rank every stored embedding under `model` against an already-computed
/// `query_vector`.
///
/// Pure database read, no network access, so a caller can embed the query
/// first and only lock the database for this part.
pub fn rank_by_similarity(
    conn: &Connection,
    query_vector: &[f32],
    model: &str,
    limit: u32,
) -> rusqlite::Result<Vec<SimilarFile>> {
    let rows = stored_embeddings(conn, model)?;

    let mut scored: Vec<SimilarFile> = rows
        .into_iter()
        .map(|row| SimilarFile {
            score: cosine_similarity(query_vector, &bytes_to_vector(&row.vector)),
            path: row.path,
            name: row.name,
            size_bytes: row.size_bytes,
            modified_at: row.modified_at,
            category: row.category,
        })
        .collect();
    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    scored.truncate(limit as usize);
    Ok(scored)
}

fn embedding_text(name: &str, category: Option<&str>, parent: &str) -> String {
    let folder = parent
        .rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(parent);
    category.map_or_else(
        || format!("{name} in folder {folder}"),
        |cat| format!("{name} ({cat}) in folder {folder}"),
    )
}

/// `(embedded, pending)` counts of live files under `model`, for a status
/// display ("12,003 of 26,790 files indexed") without loading any vectors.
pub fn index_status(conn: &Connection, model: &str) -> rusqlite::Result<(i64, i64)> {
    let embedded: i64 = conn.query_row(
        "SELECT COUNT(*) FROM file_embeddings fe
         JOIN files f ON f.path = fe.path
         WHERE fe.model = ?1 AND f.removed_at IS NULL",
        rusqlite::params![model],
        |r| r.get(0),
    )?;
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM files f
         LEFT JOIN file_embeddings fe ON fe.path = f.path AND fe.model = ?1
         WHERE f.is_dir = 0 AND f.removed_at IS NULL AND fe.path IS NULL",
        rusqlite::params![model],
        |r| r.get(0),
    )?;
    Ok((embedded, pending))
}

fn pending_files(
    conn: &Connection,
    model: &str,
) -> rusqlite::Result<Vec<(String, String, Option<String>, String)>> {
    let mut stmt = conn.prepare(
        "SELECT f.path, f.name, f.category, f.parent
         FROM files f
         LEFT JOIN file_embeddings fe ON fe.path = f.path AND fe.model = ?1
         WHERE f.is_dir = 0 AND f.removed_at IS NULL AND fe.path IS NULL",
    )?;
    let rows = stmt.query_map(rusqlite::params![model], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })?;
    rows.collect()
}

struct StoredEmbeddingRow {
    path: String,
    name: String,
    size_bytes: i64,
    modified_at: Option<i64>,
    category: Option<String>,
    vector: Vec<u8>,
}

fn stored_embeddings(conn: &Connection, model: &str) -> rusqlite::Result<Vec<StoredEmbeddingRow>> {
    let mut stmt = conn.prepare(
        "SELECT fe.path, f.name, f.size_bytes, f.modified_at, f.category, fe.vector
         FROM file_embeddings fe
         JOIN files f ON f.path = fe.path
         WHERE fe.model = ?1 AND f.removed_at IS NULL",
    )?;
    let rows = stmt.query_map(rusqlite::params![model], |r| {
        Ok(StoredEmbeddingRow {
            path: r.get(0)?,
            name: r.get(1)?,
            size_bytes: r.get(2)?,
            modified_at: r.get(3)?,
            category: r.get(4)?,
            vector: r.get(5)?,
        })
    })?;
    rows.collect()
}

fn store_embedding(
    conn: &Connection,
    path: &str,
    model: &str,
    vector: &[f32],
    now: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO file_embeddings (path, model, dims, vector, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(path) DO UPDATE SET
             model = excluded.model,
             dims = excluded.dims,
             vector = excluded.vector,
             created_at = excluded.created_at",
        rusqlite::params![path, model, vector.len(), vector_to_bytes(vector), now],
    )?;
    Ok(())
}

fn vector_to_bytes(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_vector(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_db::queries::{upsert_file, upsert_volume};
    use atlas_db::{apply_migrations, open_in_memory, FileRow, VolumeRow};
    use std::sync::Mutex;

    fn make_conn() -> Connection {
        let mut c = open_in_memory().expect("open");
        apply_migrations(&mut c).expect("migrate");
        c
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_file(conn: &Connection, path: &str, parent: &str, name: &str, category: &str) {
        let tx = conn.unchecked_transaction().unwrap();
        upsert_volume(
            &tx,
            &VolumeRow {
                id: "vol:test".into(),
                label: None,
                fs_type: None,
                mount: "C:\\".into(),
                total_bytes: None,
                first_seen: 0,
                last_seen: 0,
            },
        )
        .unwrap();
        upsert_file(
            &tx,
            &FileRow {
                path: path.to_string(),
                parent: parent.to_string(),
                name: name.to_string(),
                extension: None,
                size_bytes: 100,
                created_at: Some(1),
                modified_at: Some(1),
                accessed_at: Some(1),
                hash_blake3: None,
                hash_size: None,
                category: Some(category.to_string()),
                is_dir: false,
                is_hidden: false,
                is_symlink: false,
                volume_id: "vol:test".into(),
                first_seen: 0,
                last_seen: 0,
                removed_at: None,
            },
        )
        .unwrap();
        tx.commit().unwrap();
    }

    /// A fake provider that returns a fixed vector per input text, so tests
    /// exercise the storage/scoring pipeline without a real Ollama instance.
    struct FakeEmbedder {
        model: String,
        vectors: Mutex<std::collections::HashMap<String, Vec<f32>>>,
        calls: Mutex<u32>,
    }

    impl FakeEmbedder {
        fn new(model: &str) -> Self {
            Self {
                model: model.to_string(),
                vectors: Mutex::new(std::collections::HashMap::new()),
                calls: Mutex::new(0),
            }
        }

        fn set(&self, text: &str, vector: Vec<f32>) {
            self.vectors
                .lock()
                .unwrap()
                .insert(text.to_string(), vector);
        }
    }

    impl EmbeddingProvider for FakeEmbedder {
        fn embed(&self, text: &str) -> crate::provider::Result<Vec<f32>> {
            *self.calls.lock().unwrap() += 1;
            Ok(self
                .vectors
                .lock()
                .unwrap()
                .get(text)
                .cloned()
                .unwrap_or_else(|| vec![0.0, 0.0]))
        }

        fn model_name(&self) -> &str {
            &self.model
        }
    }

    #[test]
    fn vector_bytes_roundtrip() {
        let v = vec![1.5_f32, -2.25, 0.0, 3.75];
        assert_eq!(bytes_to_vector(&vector_to_bytes(&v)), v);
    }

    #[test]
    fn cosine_similarity_of_identical_vectors_is_one() {
        let v = vec![1.0_f32, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_of_opposite_vectors_is_negative_one() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![-1.0_f32, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_handles_zero_vector_without_dividing_by_zero() {
        let a = vec![0.0_f32, 0.0];
        let b = vec![1.0_f32, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < f32::EPSILON);
    }

    #[test]
    fn build_embedding_index_only_embeds_pending_files() {
        let conn = make_conn();
        seed_file(&conn, "C:\\r\\a.txt", "C:\\r", "a.txt", "Document");
        seed_file(&conn, "C:\\r\\b.txt", "C:\\r", "b.txt", "Document");
        let provider = FakeEmbedder::new("test-model");
        let cancel = AtomicBool::new(false);

        let stats = build_embedding_index(&conn, &provider, 100, &cancel, |_| {}).unwrap();
        assert_eq!(stats.files_embedded, 2);
        assert_eq!(stats.errors, 0);
        assert_eq!(*provider.calls.lock().unwrap(), 2);

        // Running again finds nothing left to embed.
        let stats_again = build_embedding_index(&conn, &provider, 200, &cancel, |_| {}).unwrap();
        assert_eq!(stats_again.files_embedded, 0);
    }

    #[test]
    fn switching_models_makes_every_file_pending_again() {
        let conn = make_conn();
        seed_file(&conn, "C:\\r\\a.txt", "C:\\r", "a.txt", "Document");
        let cancel = AtomicBool::new(false);

        let old_provider = FakeEmbedder::new("old-model");
        build_embedding_index(&conn, &old_provider, 100, &cancel, |_| {}).unwrap();

        let new_provider = FakeEmbedder::new("new-model");
        let stats = build_embedding_index(&conn, &new_provider, 200, &cancel, |_| {}).unwrap();
        assert_eq!(
            stats.files_embedded, 1,
            "new model has no rows yet, so the file is pending again"
        );
    }

    #[test]
    fn semantic_search_ranks_by_similarity() {
        let conn = make_conn();
        seed_file(
            &conn,
            "C:\\r\\invoice.pdf",
            "C:\\r",
            "invoice.pdf",
            "Document",
        );
        seed_file(
            &conn,
            "C:\\r\\vacation.jpg",
            "C:\\r",
            "vacation.jpg",
            "Image",
        );

        let provider = FakeEmbedder::new("test-model");
        provider.set(
            &embedding_text("invoice.pdf", Some("Document"), "C:\\r"),
            vec![1.0, 0.0],
        );
        provider.set(
            &embedding_text("vacation.jpg", Some("Image"), "C:\\r"),
            vec![0.0, 1.0],
        );
        provider.set("financial paperwork", vec![1.0, 0.0]);

        let cancel = AtomicBool::new(false);
        build_embedding_index(&conn, &provider, 100, &cancel, |_| {}).unwrap();

        let results = semantic_search(&conn, &provider, "financial paperwork", 5).unwrap();
        assert_eq!(results[0].path, "C:\\r\\invoice.pdf");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn removed_files_are_excluded_from_semantic_search() {
        let conn = make_conn();
        seed_file(&conn, "C:\\r\\a.txt", "C:\\r", "a.txt", "Document");
        let provider = FakeEmbedder::new("test-model");
        let cancel = AtomicBool::new(false);
        build_embedding_index(&conn, &provider, 100, &cancel, |_| {}).unwrap();

        conn.execute(
            "UPDATE files SET removed_at = 999 WHERE path = 'C:\\r\\a.txt'",
            [],
        )
        .unwrap();

        let results = semantic_search(&conn, &provider, "a.txt in folder r", 5).unwrap();
        assert!(results.is_empty());
    }
}
