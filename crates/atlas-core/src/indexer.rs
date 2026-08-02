//! Consumes `ScanEvent`s from the scanner channel and persists them into the
//! SQLite index in batches. The indexer is the single writer to the DB.
//!
//! Contract:
//!
//! - One call to `run` corresponds to one scan session.
//! - The indexer opens a `scans` row at the start, updates it as progress
//!   arrives, and closes it (completed / cancelled / failed) when the channel
//!   drains.
//! - After every `BATCH_SIZE` `Entry` events, the batched transaction is
//!   committed and a new one is started.
//! - At the end of a scan, `mark_removed_since` sweeps stale rows under the
//!   scan root that were not touched by this scan.

use std::path::Path;

use crossbeam_channel::Receiver;
use thiserror::Error;
use time::OffsetDateTime;
use tracing::warn;

use atlas_db::queries::{count_live_files, mark_removed_since, upsert_file, upsert_volume};
use atlas_db::{FileRow, VolumeRow};

use crate::scanner::{ScanEvent, ScanReport};

const BATCH_SIZE: usize = 500;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error("scan finished with no Done event")]
    NoDoneEvent,
}

pub type Result<T> = std::result::Result<T, IndexError>;

/// Metadata for a single scan session.
#[derive(Debug, Clone)]
pub struct ScanMeta {
    pub root: String,
    pub volume_id: String,
    pub scan_ts: i64,
}

impl ScanMeta {
    pub fn now(root: impl Into<String>, volume_id: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            volume_id: volume_id.into(),
            scan_ts: OffsetDateTime::now_utc().unix_timestamp(),
        }
    }
}

/// Statistics reported at the end of `run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStats {
    pub entries_persisted: u64,
    pub errors: u32,
    pub removed_marked: usize,
    pub live_files_after: i64,
    pub scan_id: i64,
    pub scan_report: Option<ScanReport>,
}

/// Persist a single volume row before or during a scan. Idempotent.
pub fn record_volume(conn: &mut rusqlite::Connection, volume: &VolumeRow) -> Result<()> {
    let tx = conn.transaction()?;
    upsert_volume(&tx, volume)?;
    tx.commit()?;
    Ok(())
}

/// Live progress forwarded to `run_with_progress`'s callback as entries and
/// scanner progress ticks arrive. Distinct from `ScanReport`, which is only
/// available once the scan finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexProgress {
    pub files_seen: u64,
    pub bytes_seen: u64,
}

/// Consume every event from `rx`, persist entries, and return statistics.
pub fn run(
    conn: &mut rusqlite::Connection,
    rx: Receiver<ScanEvent>,
    meta: &ScanMeta,
) -> Result<IndexStats> {
    run_with_progress(conn, rx, meta, |_| {})
}

/// Same contract as `run`, but invokes `on_progress` for every scanner
/// progress tick so a caller (e.g. the Tauri command layer) can forward live
/// updates to a UI without polling the database.
pub fn run_with_progress(
    conn: &mut rusqlite::Connection,
    rx: Receiver<ScanEvent>,
    meta: &ScanMeta,
    mut on_progress: impl FnMut(IndexProgress),
) -> Result<IndexStats> {
    let scan_id = open_scan_row(conn, meta)?;
    let mut entries_persisted: u64 = 0;
    let mut errors: u32 = 0;
    let mut buffer: Vec<FileRow> = Vec::with_capacity(BATCH_SIZE);
    let mut last_report: Option<ScanReport> = None;

    for event in rx {
        match event {
            ScanEvent::Entry(record) => {
                let category = crate::classifier::classify(&record.path, record.is_dir);
                buffer.push(record.to_file_row(meta.scan_ts, Some(category.as_str().to_string())));
                if buffer.len() >= BATCH_SIZE {
                    entries_persisted += flush(conn, &buffer)?;
                    buffer.clear();
                }
            }
            ScanEvent::Progress {
                files_seen,
                bytes_seen,
            } => {
                on_progress(IndexProgress {
                    files_seen,
                    bytes_seen,
                });
            }
            ScanEvent::Error { path, message } => {
                errors += 1;
                warn!(path = %path.display(), error = %message, "scanner reported error");
            }
            ScanEvent::Done(report) => {
                last_report = Some(report);
            }
        }
    }

    if !buffer.is_empty() {
        entries_persisted += flush(conn, &buffer)?;
    }

    let now = OffsetDateTime::now_utc().unix_timestamp();
    let removed_marked = sweep_removed(conn, &meta.root, meta.scan_ts, now)?;
    let live_files_after = count_live_files(conn)?;

    close_scan_row(
        conn,
        scan_id,
        entries_persisted,
        last_report.as_ref(),
        now,
        errors,
    )?;

    Ok(IndexStats {
        entries_persisted,
        errors,
        removed_marked,
        live_files_after,
        scan_id,
        scan_report: last_report,
    })
}

fn open_scan_row(conn: &rusqlite::Connection, meta: &ScanMeta) -> Result<i64> {
    conn.execute(
        "INSERT INTO scans (root, started_at, status) VALUES (?1, ?2, 'running')",
        rusqlite::params![meta.root, meta.scan_ts],
    )?;
    Ok(conn.last_insert_rowid())
}

fn close_scan_row(
    conn: &rusqlite::Connection,
    scan_id: i64,
    files_persisted: u64,
    report: Option<&ScanReport>,
    finished_at: i64,
    error_count: u32,
) -> Result<()> {
    let (status, bytes_seen) = match report {
        Some(r) if r.cancelled => ("cancelled", r.bytes_seen),
        Some(r) => ("completed", r.bytes_seen),
        None => (
            if error_count > 0 {
                "failed"
            } else {
                "completed"
            },
            0,
        ),
    };
    conn.execute(
        "UPDATE scans SET finished_at = ?1, files_seen = ?2, bytes_seen = ?3, status = ?4 WHERE id = ?5",
        rusqlite::params![
            finished_at,
            i64::try_from(files_persisted).unwrap_or(i64::MAX),
            i64::try_from(bytes_seen).unwrap_or(i64::MAX),
            status,
            scan_id,
        ],
    )?;
    Ok(())
}

fn flush(conn: &mut rusqlite::Connection, rows: &[FileRow]) -> Result<u64> {
    let tx = conn.transaction()?;
    for row in rows {
        upsert_file(&tx, row)?;
    }
    tx.commit()?;
    Ok(rows.len() as u64)
}

fn sweep_removed(
    conn: &mut rusqlite::Connection,
    root: &str,
    scan_ts: i64,
    now: i64,
) -> Result<usize> {
    let tx = conn.transaction()?;
    let n = mark_removed_since(&tx, root, scan_ts, now)?;
    tx.commit()?;
    Ok(n)
}

/// Convenience: derive a canonical root prefix from a `Path`.
#[must_use]
pub fn root_prefix(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{scan, ScanConfig};
    use crate::skip_rules::SkipRules;
    use atlas_db::models::VolumeRow;
    use atlas_db::{apply_migrations, open_in_memory};
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    fn make_conn() -> rusqlite::Connection {
        let mut c = open_in_memory().expect("open");
        apply_migrations(&mut c).expect("migrate");
        c
    }

    fn seed_volume(conn: &mut rusqlite::Connection) {
        record_volume(
            conn,
            &VolumeRow {
                id: "vol:test".into(),
                label: Some("Test".into()),
                fs_type: Some("NTFS".into()),
                mount: "C:\\".into(),
                total_bytes: None,
                first_seen: 0,
                last_seen: 0,
            },
        )
        .expect("seed volume");
    }

    #[test]
    fn full_scan_persists_every_entry() {
        let root = tempdir().expect("tempdir");
        fs::create_dir_all(root.path().join("a")).unwrap();
        fs::write(root.path().join("a/one.txt"), b"hello").unwrap();
        fs::write(root.path().join("two.txt"), b"world").unwrap();

        let (tx, rx) = crossbeam_channel::unbounded();
        let cancel = AtomicBool::new(false);
        let cfg = ScanConfig {
            root: root.path().to_path_buf(),
            volume_id: "vol:test".into(),
            rules: SkipRules::permissive(),
        };
        std::thread::spawn(move || {
            scan(&cfg, &cancel, &tx);
        });

        let mut conn = make_conn();
        seed_volume(&mut conn);
        let meta = ScanMeta::now(root_prefix(root.path()), "vol:test");
        let stats = run(&mut conn, rx, &meta).expect("index");

        assert_eq!(stats.entries_persisted, 4); // root, a, one.txt, two.txt
        assert_eq!(stats.errors, 0);
        assert_eq!(stats.removed_marked, 0);
        assert!(stats.live_files_after >= 4);
        assert!(stats.scan_report.is_some());
    }

    #[test]
    fn rescan_marks_deleted_files_as_removed() {
        let root = tempdir().expect("tempdir");
        fs::write(root.path().join("keep.txt"), b"a").unwrap();
        fs::write(root.path().join("delete_me.txt"), b"b").unwrap();

        let mut conn = make_conn();
        seed_volume(&mut conn);
        let prefix = root_prefix(root.path());

        // Scan 1
        let (tx1, rx1) = crossbeam_channel::unbounded();
        let cancel1 = AtomicBool::new(false);
        let cfg = ScanConfig {
            root: root.path().to_path_buf(),
            volume_id: "vol:test".into(),
            rules: SkipRules::permissive(),
        };
        {
            #[allow(clippy::redundant_clone)]
            let cfg_c = cfg.clone();
            std::thread::spawn(move || {
                let _ = scan(&cfg_c, &cancel1, &tx1);
            });
        }
        let meta1 = ScanMeta::now(prefix.clone(), "vol:test");
        run(&mut conn, rx1, &meta1).unwrap();

        // Sleep to advance scan_ts
        std::thread::sleep(std::time::Duration::from_millis(1_100));

        // Delete a file and rescan
        fs::remove_file(root.path().join("delete_me.txt")).unwrap();
        let (tx2, rx2) = crossbeam_channel::unbounded();
        let cancel2 = AtomicBool::new(false);
        {
            #[allow(clippy::redundant_clone)]
            let cfg_c = cfg.clone();
            std::thread::spawn(move || {
                let _ = scan(&cfg_c, &cancel2, &tx2);
            });
        }
        let meta2 = ScanMeta::now(prefix, "vol:test");
        let stats = run(&mut conn, rx2, &meta2).unwrap();

        assert_eq!(stats.removed_marked, 1);
    }
}
