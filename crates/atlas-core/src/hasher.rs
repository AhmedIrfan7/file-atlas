//! BLAKE3 content hashing, size-gated for duplicate detection.
//!
//! Hashing every indexed file would be wasteful: a file can only be a
//! duplicate of another file the same size, so we only hash files whose
//! size collides with at least one other live file. This is the "size-gated"
//! step referenced in the roadmap. Hashing itself streams the file through a
//! fixed-size buffer rather than reading it into memory, so a multi-gigabyte
//! file does not blow up process memory.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Files smaller than this are never hashed. Duplicate detection on
/// near-empty files (icons, `.gitkeep`, placeholders) is rarely useful and
/// they are common enough to waste real time hashing at scale.
pub const MIN_HASH_SIZE_BYTES: i64 = 1024;

const READ_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashProgress {
    pub files_hashed: u64,
    pub files_total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashStats {
    pub files_hashed: u64,
    pub errors: u64,
}

/// Hash every live file whose size collides with at least one other file.
///
/// Only files that have not been hashed yet are considered. `on_progress` is
/// invoked after each file (hashed or errored) so a caller can show a live
/// counter.
pub fn hash_pending_duplicates(
    conn: &Connection,
    cancel: &std::sync::atomic::AtomicBool,
    mut on_progress: impl FnMut(HashProgress),
) -> rusqlite::Result<HashStats> {
    let candidates = collision_candidates(conn)?;
    let files_total = candidates.len() as u64;
    let mut files_hashed = 0u64;
    let mut errors = 0u64;

    for (id, path) in candidates {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        match hash_file(Path::new(&path)) {
            Ok(hash) => {
                store_hash(conn, id, &hash)?;
                files_hashed += 1;
            }
            Err(err) => {
                tracing::warn!(path = %path, error = %err, "hash failed");
                errors += 1;
            }
        }
        on_progress(HashProgress {
            files_hashed: files_hashed + errors,
            files_total,
        });
    }

    Ok(HashStats {
        files_hashed,
        errors,
    })
}

/// `(id, path)` pairs for live files that are not yet hashed and whose size
/// is shared with at least one other live file.
fn collision_candidates(conn: &Connection) -> rusqlite::Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.path
         FROM files f
         WHERE f.removed_at IS NULL
           AND f.is_dir = 0
           AND f.hash_blake3 IS NULL
           AND f.size_bytes >= ?1
           AND f.size_bytes IN (
               SELECT size_bytes FROM files
               WHERE removed_at IS NULL AND is_dir = 0 AND size_bytes >= ?1
               GROUP BY size_bytes
               HAVING COUNT(*) > 1
           )
         ORDER BY f.size_bytes DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![MIN_HASH_SIZE_BYTES], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    rows.collect()
}

fn store_hash(conn: &Connection, id: i64, hash: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE files SET hash_blake3 = ?1 WHERE id = ?2",
        rusqlite::params![hash, id],
    )?;
    Ok(())
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; READ_BUFFER_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_db::queries::{upsert_file, upsert_volume};
    use atlas_db::{apply_migrations, open_in_memory, FileRow, VolumeRow};
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    fn make_conn() -> Connection {
        let mut c = open_in_memory().expect("open");
        apply_migrations(&mut c).expect("migrate");
        c
    }

    fn seed_file(conn: &Connection, path: &str, size: i64) {
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
                parent: "C:\\r".into(),
                name: path.rsplit(['\\', '/']).next().unwrap_or(path).to_string(),
                extension: None,
                size_bytes: size,
                created_at: Some(1),
                modified_at: Some(1),
                accessed_at: Some(1),
                hash_blake3: None,
                hash_size: None,
                category: Some("Other".into()),
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

    #[test]
    fn hashes_only_size_colliding_files() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        let unique = dir.path().join("unique.bin");
        fs::write(&a, vec![1u8; 2048]).unwrap();
        fs::write(&b, vec![2u8; 2048]).unwrap();
        fs::write(&unique, vec![3u8; 4096]).unwrap();

        let conn = make_conn();
        seed_file(&conn, a.to_str().unwrap(), 2048);
        seed_file(&conn, b.to_str().unwrap(), 2048);
        seed_file(&conn, unique.to_str().unwrap(), 4096);

        let cancel = AtomicBool::new(false);
        let stats = hash_pending_duplicates(&conn, &cancel, |_| {}).unwrap();
        assert_eq!(stats.files_hashed, 2);
        assert_eq!(stats.errors, 0);

        let unique_hash: Option<String> = conn
            .query_row(
                "SELECT hash_blake3 FROM files WHERE path = ?1",
                rusqlite::params![unique.to_str().unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        assert!(unique_hash.is_none(), "singleton size should not be hashed");
    }

    #[test]
    fn identical_content_produces_identical_hash() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        let content = vec![7u8; 2048];
        fs::write(&a, &content).unwrap();
        fs::write(&b, &content).unwrap();

        let conn = make_conn();
        seed_file(&conn, a.to_str().unwrap(), 2048);
        seed_file(&conn, b.to_str().unwrap(), 2048);

        let cancel = AtomicBool::new(false);
        hash_pending_duplicates(&conn, &cancel, |_| {}).unwrap();

        let hash_a: String = conn
            .query_row(
                "SELECT hash_blake3 FROM files WHERE path = ?1",
                rusqlite::params![a.to_str().unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        let hash_b: String = conn
            .query_row(
                "SELECT hash_blake3 FROM files WHERE path = ?1",
                rusqlite::params![b.to_str().unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hash_a, hash_b);
    }

    #[test]
    fn tiny_files_below_threshold_are_skipped() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.tiny");
        let b = dir.path().join("b.tiny");
        fs::write(&a, b"x").unwrap();
        fs::write(&b, b"y").unwrap();

        let conn = make_conn();
        seed_file(&conn, a.to_str().unwrap(), 1);
        seed_file(&conn, b.to_str().unwrap(), 1);

        let cancel = AtomicBool::new(false);
        let stats = hash_pending_duplicates(&conn, &cancel, |_| {}).unwrap();
        assert_eq!(stats.files_hashed, 0);
    }

    #[test]
    fn missing_file_on_disk_is_counted_as_error_not_panic() {
        let conn = make_conn();
        seed_file(&conn, "C:\\definitely\\does\\not\\exist_a.bin", 2048);
        seed_file(&conn, "C:\\definitely\\does\\not\\exist_b.bin", 2048);

        let cancel = AtomicBool::new(false);
        let stats = hash_pending_duplicates(&conn, &cancel, |_| {}).unwrap();
        assert_eq!(stats.files_hashed, 0);
        assert_eq!(stats.errors, 2);
    }
}
