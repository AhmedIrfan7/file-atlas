//! Tauri command handlers for hashing, duplicate detection, and the
//! trash/restore side of the safety pipeline. Same thin-adapter contract as
//! `commands.rs`.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use atlas_core::{
    find_duplicate_groups, hash_pending_duplicates, list_recent_trash_actions, restore_action,
    trash_paths, DuplicateGroup, HashProgress, HashStats, RestoreOutcome, TrashOutcome,
};
use atlas_db::ActionRow;
use atlas_platform::current;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct HashProgressEvent {
    pub files_hashed: u64,
    pub files_total: u64,
}

/// Start hashing every live file whose size collides with another file.
/// Returns immediately; progress arrives via `hash-progress` and
/// `hash-finished` events, matching the `start_scan` pattern.
#[tauri::command]
pub fn hash_duplicates(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.hash_running.swap(true, Ordering::SeqCst) {
        return Err("A hash pass is already running.".into());
    }
    state.hash_cancel.store(false, Ordering::SeqCst);

    let cancel = Arc::clone(&state.hash_cancel);
    let running = Arc::clone(&state.hash_running);
    let app_for_thread = app.clone();

    std::thread::spawn(move || {
        let state = app_for_thread.state::<AppState>();
        let conn = state.db.lock();
        let app_for_events = app_for_thread.clone();
        let result =
            hash_pending_duplicates(&conn, cancel.as_ref(), move |progress: HashProgress| {
                let _ = app_for_events.emit(
                    "hash-progress",
                    HashProgressEvent {
                        files_hashed: progress.files_hashed,
                        files_total: progress.files_total,
                    },
                );
            });
        drop(conn);
        running.store(false, Ordering::SeqCst);
        let hash_stats = result.unwrap_or(HashStats {
            files_hashed: 0,
            errors: 0,
        });
        let _ = app_for_thread.emit("hash-finished", hash_stats);
    });

    Ok(())
}

/// Cancel an in-progress hash pass. No-op if none is running.
#[tauri::command]
pub fn cancel_hash(state: State<'_, AppState>) {
    state.hash_cancel.store(true, Ordering::SeqCst);
}

/// The `limit` duplicate groups with the most wasted space.
#[tauri::command]
pub fn get_duplicate_groups(
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateGroup>, String> {
    let conn = state.db.lock();
    find_duplicate_groups(&conn, limit).map_err(|e| e.to_string())
}

/// Send `paths` to the OS trash, guardrails permitting. The UI is expected
/// to have already shown a preview and gotten explicit confirmation before
/// calling this; this command performs the actual, no-more-questions-asked
/// execution step of the safety pipeline.
#[tauri::command]
pub fn trash_selected_paths(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TrashOutcome>, String> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let platform = current();
    let conn = state.db.lock();
    trash_paths(&conn, &platform, &paths, now).map_err(|e| e.to_string())
}

/// Restore a single previously trashed file by its `actions_log` id.
#[tauri::command]
pub fn restore_trash_action(
    action_id: i64,
    state: State<'_, AppState>,
) -> Result<RestoreOutcome, String> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let platform = current();
    let conn = state.db.lock();
    restore_action(&conn, &platform, action_id, now).map_err(|e| e.to_string())
}

/// The `limit` most recent trash actions, for a "recently deleted" panel.
#[tauri::command]
pub fn list_recent_actions(
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ActionRow>, String> {
    let conn = state.db.lock();
    list_recent_trash_actions(&conn, limit).map_err(|e| e.to_string())
}
