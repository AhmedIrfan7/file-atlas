//! Tauri command handlers.
//!
//! Every handler here is a thin adapter: it deserializes arguments, calls
//! into `atlas-core` / `atlas-db` / `atlas-platform`, and serializes the
//! result. No business logic lives in this file.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use atlas_core::{
    default_roots, home_summary, index_run_with_progress, record_volume, root_prefix, scan,
    stale_bucket, top_largest, top_oldest, FileSummary, HomeSummary, IndexProgress, ScanConfig,
    ScanMeta, SkipRules, StaleBucket, SuggestedRoot,
};
use atlas_platform::PlatformFs;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

use crate::state::AppState;
use crate::volume::{resolve_volume_for, to_volume_row};

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgressEvent {
    pub root: String,
    pub files_seen: u64,
    pub bytes_seen: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanFinishedEvent {
    pub roots_scanned: usize,
    pub total_entries_persisted: u64,
    pub total_removed_marked: usize,
    pub total_errors: u32,
    pub cancelled: bool,
}

/// Suggested onboarding roots (Desktop, Downloads, Documents, Pictures,
/// Videos, Music) that exist on this machine.
#[tauri::command]
pub fn get_default_roots() -> Vec<SuggestedRoot> {
    default_roots()
}

/// Kick off a background scan of every root in `roots`. Emits `scan-progress`
/// events while running and a single `scan-finished` event at the end.
/// Returns immediately; the caller listens for events rather than awaiting.
#[tauri::command]
pub fn start_scan(roots: Vec<String>, app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.scan_running.swap(true, Ordering::SeqCst) {
        return Err("A scan is already running.".into());
    }
    state.scan_cancel.store(false, Ordering::SeqCst);

    let cancel = Arc::clone(&state.scan_cancel);
    let running = Arc::clone(&state.scan_running);
    let app_for_thread = app.clone();

    std::thread::spawn(move || {
        let mut total_entries_persisted = 0u64;
        let mut total_removed_marked = 0usize;
        let mut total_errors = 0u32;
        let mut cancelled = false;

        for root in &roots {
            if cancel.load(Ordering::Relaxed) {
                cancelled = true;
                break;
            }
            match scan_one_root(&app_for_thread, root, &cancel) {
                Ok(root_stats) => {
                    total_entries_persisted += root_stats.entries_persisted;
                    total_removed_marked += root_stats.removed_marked;
                    total_errors += root_stats.errors;
                    if root_stats.scan_report.as_ref().is_some_and(|r| r.cancelled) {
                        cancelled = true;
                        break;
                    }
                }
                Err(err) => {
                    tracing::error!(root = %root, error = %err, "scan failed");
                    total_errors += 1;
                }
            }
        }

        running.store(false, Ordering::SeqCst);
        let _ = app_for_thread.emit(
            "scan-finished",
            ScanFinishedEvent {
                roots_scanned: roots.len(),
                total_entries_persisted,
                total_removed_marked,
                total_errors,
                cancelled,
            },
        );
    });

    Ok(())
}

fn scan_one_root(
    app: &AppHandle,
    root: &str,
    cancel: &Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<atlas_core::IndexStats> {
    let path = PathBuf::from(root);
    let volume = resolve_volume_for(&path);
    let volume_id = volume.id.clone();

    let state = app.state::<AppState>();
    {
        let mut conn = state.db.lock();
        record_volume(&mut conn, &to_volume_row(&volume))?;
    }

    let (tx, rx) = crossbeam_channel::unbounded();
    let cfg = ScanConfig {
        root: path.clone(),
        volume_id: volume_id.clone(),
        rules: SkipRules::default(),
    };
    let cancel_bg = Arc::clone(cancel);
    let scan_handle = std::thread::spawn(move || {
        let _ = scan(&cfg, cancel_bg.as_ref(), &tx);
    });

    let root_for_events = root.to_string();
    let app_for_events = app.clone();
    let meta = ScanMeta::now(root_prefix(&path), volume_id);
    let index_stats = {
        let mut conn = state.db.lock();
        index_run_with_progress(&mut conn, rx, &meta, move |progress: IndexProgress| {
            let _ = app_for_events.emit(
                "scan-progress",
                ScanProgressEvent {
                    root: root_for_events.clone(),
                    files_seen: progress.files_seen,
                    bytes_seen: progress.bytes_seen,
                },
            );
        })?
    };

    scan_handle.join().expect("scanner thread panicked");
    Ok(index_stats)
}

/// Request cancellation of any scan currently in progress. A no-op if no
/// scan is running.
#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) {
    state.scan_cancel.store(true, Ordering::SeqCst);
}

/// Whether a scan is currently in progress.
#[tauri::command]
pub fn is_scanning(state: State<'_, AppState>) -> bool {
    state.scan_running.load(Ordering::SeqCst)
}

/// Aggregate totals plus the category breakdown for the home view.
#[tauri::command]
pub fn get_home_summary(state: State<'_, AppState>) -> Result<HomeSummary, String> {
    let conn = state.db.lock();
    home_summary(&conn).map_err(|e| e.to_string())
}

/// The `limit` largest indexed files.
#[tauri::command]
pub fn get_top_largest(limit: u32, state: State<'_, AppState>) -> Result<Vec<FileSummary>, String> {
    let conn = state.db.lock();
    top_largest(&conn, limit).map_err(|e| e.to_string())
}

/// The `limit` least-recently-modified indexed files.
#[tauri::command]
pub fn get_top_oldest(limit: u32, state: State<'_, AppState>) -> Result<Vec<FileSummary>, String> {
    let conn = state.db.lock();
    top_oldest(&conn, limit).map_err(|e| e.to_string())
}

/// Files not modified in at least `min_age_days`, with up to `sample_limit`
/// example files.
#[tauri::command]
pub fn get_stale_bucket(
    min_age_days: u32,
    sample_limit: u32,
    state: State<'_, AppState>,
) -> Result<StaleBucket, String> {
    let conn = state.db.lock();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    stale_bucket(&conn, now, min_age_days, sample_limit).map_err(|e| e.to_string())
}

/// Reveal `path` in the OS file manager: Explorer on Windows, Finder on
/// macOS, the default file manager via `xdg-open` on Linux.
#[tauri::command]
pub fn open_in_file_manager(path: String) -> Result<(), String> {
    atlas_platform::current()
        .open_in_file_manager(std::path::Path::new(&path))
        .map_err(|e| e.to_string())
}
