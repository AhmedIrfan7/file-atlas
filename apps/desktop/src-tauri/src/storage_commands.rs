//! Tauri command handler for the storage map (treemap) view. Same
//! thin-adapter contract as `commands.rs`: purely a read, no execution path
//! of its own.

use atlas_core::{get_storage_map, StorageMapFilter, StorageMapResponse};
use tauri::State;
use time::OffsetDateTime;

use crate::state::AppState;

const SECONDS_PER_DAY: i64 = 86_400;

/// The treemap view for `path` (top-level scan roots if `None`), optionally
/// narrowed to one category and/or to files modified in the last
/// `since_days` days.
#[tauri::command]
pub fn get_storage_map_view(
    path: Option<String>,
    category: Option<String>,
    since_days: Option<u32>,
    state: State<'_, AppState>,
) -> Result<StorageMapResponse, String> {
    let since_unix = since_days
        .map(|days| OffsetDateTime::now_utc().unix_timestamp() - i64::from(days) * SECONDS_PER_DAY);
    let filter = StorageMapFilter {
        category,
        since_unix,
    };
    let conn = state.db.lock();
    get_storage_map(&conn, path.as_deref(), &filter).map_err(|e| e.to_string())
}
