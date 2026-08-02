//! Filesystem traversal that emits `FileRecord` events into a channel.
//!
//! The scanner is intentionally simple. It walks with `walkdir`, applies the
//! configured `SkipRules`, respects a cancellation flag, and reports progress
//! every `PROGRESS_INTERVAL` entries. It never touches the database directly;
//! the indexer consumes the channel and handles persistence.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use tracing::warn;
use walkdir::WalkDir;

use crate::file_record::FileRecord;
use crate::skip_rules::SkipRules;

const PROGRESS_INTERVAL: u64 = 500;

/// Inputs to a single scan run.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub root: PathBuf,
    pub volume_id: String,
    pub rules: SkipRules,
}

/// Summary of a completed scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    pub files_seen: u64,
    pub bytes_seen: u64,
    pub errors: u32,
    pub duration_ms: u128,
    pub cancelled: bool,
}

/// Events streamed on the scanner channel.
#[derive(Debug, Clone)]
pub enum ScanEvent {
    Entry(FileRecord),
    Progress { files_seen: u64, bytes_seen: u64 },
    Error { path: PathBuf, message: String },
    Done(ScanReport),
}

/// Walk `config.root` and emit events on `tx`. Runs synchronously on the
/// caller thread; the caller is expected to spawn this on a worker thread.
///
/// Returns the same `ScanReport` that was sent as the final `Done` event.
pub fn scan(config: &ScanConfig, cancel: &AtomicBool, tx: &Sender<ScanEvent>) -> ScanReport {
    let started = Instant::now();
    let mut files_seen: u64 = 0;
    let mut bytes_seen: u64 = 0;
    let mut errors: u32 = 0;

    let walker = WalkDir::new(&config.root)
        .follow_links(!config.rules.skip_symlinks)
        .same_file_system(false);

    let mut iter = walker.into_iter();
    while let Some(next) = iter.next() {
        if cancel.load(Ordering::Relaxed) {
            let report = build_report(started.elapsed(), files_seen, bytes_seen, errors, true);
            let _ = tx.send(ScanEvent::Done(report.clone()));
            return report;
        }

        let entry = match next {
            Ok(e) => e,
            Err(err) => {
                errors += 1;
                let path = err
                    .path()
                    .map(std::path::Path::to_path_buf)
                    .unwrap_or_default();
                let _ = tx.send(ScanEvent::Error {
                    path,
                    message: err.to_string(),
                });
                continue;
            }
        };

        let file_name_str = entry.file_name().to_string_lossy();
        let is_hidden = file_name_str.starts_with('.')
            && file_name_str.as_ref() != "."
            && file_name_str.as_ref() != "..";

        // Root entry is always allowed; skip rules apply to descendants only.
        if entry.depth() > 0 {
            if entry.file_type().is_dir() {
                if !config
                    .rules
                    .allow_dir(entry.path(), &file_name_str, is_hidden)
                {
                    iter.skip_current_dir();
                    continue;
                }
            } else if !config
                .rules
                .allow_file(is_hidden, entry.file_type().is_symlink())
            {
                continue;
            }
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(err) => {
                errors += 1;
                warn!(path = %entry.path().display(), error = %err, "stat failed");
                let _ = tx.send(ScanEvent::Error {
                    path: entry.path().to_path_buf(),
                    message: err.to_string(),
                });
                continue;
            }
        };

        let record = FileRecord::from_metadata(entry.path(), &meta, &config.volume_id, is_hidden);
        if !record.is_dir {
            bytes_seen = bytes_seen.saturating_add(record.size_bytes);
        }
        files_seen += 1;

        if tx.send(ScanEvent::Entry(record)).is_err() {
            // Receiver dropped. Treat as cancellation.
            let report = build_report(started.elapsed(), files_seen, bytes_seen, errors, true);
            return report;
        }

        if files_seen % PROGRESS_INTERVAL == 0 {
            let _ = tx.send(ScanEvent::Progress {
                files_seen,
                bytes_seen,
            });
        }
    }

    let report = build_report(started.elapsed(), files_seen, bytes_seen, errors, false);
    let _ = tx.send(ScanEvent::Done(report.clone()));
    report
}

const fn build_report(
    elapsed: Duration,
    files_seen: u64,
    bytes_seen: u64,
    errors: u32,
    cancelled: bool,
) -> ScanReport {
    ScanReport {
        files_seen,
        bytes_seen,
        errors,
        duration_ms: elapsed.as_millis(),
        cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    fn drain(rx: crossbeam_channel::Receiver<ScanEvent>) -> Vec<ScanEvent> {
        rx.into_iter().collect()
    }

    #[test]
    fn scanner_emits_every_file_in_a_tree() {
        let root = tempdir().expect("tempdir");
        fs::create_dir_all(root.path().join("a/b")).unwrap();
        fs::write(root.path().join("a/one.txt"), b"hello").unwrap();
        fs::write(root.path().join("a/b/two.txt"), b"world").unwrap();

        let (tx, rx) = crossbeam_channel::unbounded();
        let cancel = AtomicBool::new(false);
        let cfg = ScanConfig {
            root: root.path().to_path_buf(),
            volume_id: "vol:test".into(),
            rules: SkipRules::permissive(),
        };
        let report = scan(&cfg, &cancel, &tx);
        drop(tx);

        let events = drain(rx);
        let file_count = events
            .iter()
            .filter(|e| matches!(e, ScanEvent::Entry(r) if !r.is_dir))
            .count();
        assert_eq!(file_count, 2);
        assert_eq!(report.files_seen, 5); // root, a, b, one.txt, two.txt
        assert_eq!(report.bytes_seen, 10);
        assert!(!report.cancelled);
    }

    #[test]
    fn scanner_respects_node_modules_skip() {
        let root = tempdir().expect("tempdir");
        fs::create_dir_all(root.path().join("node_modules/deep")).unwrap();
        fs::write(root.path().join("node_modules/deep/lib.js"), b"x").unwrap();
        fs::write(root.path().join("keep.txt"), b"ok").unwrap();

        let (tx, rx) = crossbeam_channel::unbounded();
        let cancel = AtomicBool::new(false);
        let cfg = ScanConfig {
            root: root.path().to_path_buf(),
            volume_id: "vol:test".into(),
            rules: SkipRules::default(),
        };
        scan(&cfg, &cancel, &tx);
        drop(tx);

        let files: Vec<String> = drain(rx)
            .into_iter()
            .filter_map(|e| match e {
                ScanEvent::Entry(r) if !r.is_dir => Some(r.name),
                _ => None,
            })
            .collect();
        assert!(files.contains(&"keep.txt".to_string()));
        assert!(!files.iter().any(|n| n == "lib.js"));
    }

    #[test]
    fn scanner_stops_when_cancelled() {
        let root = tempdir().expect("tempdir");
        for i in 0..50 {
            fs::write(root.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        let (tx, rx) = crossbeam_channel::unbounded();
        let cancel = AtomicBool::new(true); // pre-cancelled
        let cfg = ScanConfig {
            root: root.path().to_path_buf(),
            volume_id: "vol:test".into(),
            rules: SkipRules::permissive(),
        };
        let report = scan(&cfg, &cancel, &tx);
        drop(tx);
        assert!(report.cancelled);
        // Done should still be emitted
        let events: Vec<_> = rx.into_iter().collect();
        assert!(matches!(events.last(), Some(ScanEvent::Done(_))));
    }

    #[test]
    fn scanner_emits_progress_events_after_interval() {
        let root = tempdir().expect("tempdir");
        for i in 0..600 {
            fs::write(root.path().join(format!("f{i:04}.txt")), b"x").unwrap();
        }
        let (tx, rx) = crossbeam_channel::unbounded();
        let cancel = AtomicBool::new(false);
        let cfg = ScanConfig {
            root: root.path().to_path_buf(),
            volume_id: "vol:test".into(),
            rules: SkipRules::permissive(),
        };
        scan(&cfg, &cancel, &tx);
        drop(tx);

        let progress_count = rx
            .into_iter()
            .filter(|e| matches!(e, ScanEvent::Progress { .. }))
            .count();
        assert!(progress_count >= 1);
    }
}
