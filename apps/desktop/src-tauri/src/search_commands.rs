//! Tauri command handlers for search and saved searches. Same thin-adapter
//! contract as `commands.rs`.

use atlas_search::{parse, saved, search, SavedSearch, SearchHit};
use tauri::State;
use time::OffsetDateTime;

use crate::state::AppState;

/// Run `query_text` (the filter DSL from `atlas_search::parser`) against the
/// index and return up to `limit` hits.
#[tauri::command]
pub fn search_files(
    query_text: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<SearchHit>, String> {
    let query = parse(&query_text).map_err(|e| e.to_string())?;
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let conn = state.db.lock();
    search(&conn, &query, now, limit).map_err(|e| e.to_string())
}

/// Save `query_text` under `name`, replacing any existing saved search with
/// the same name.
#[tauri::command]
pub fn save_search(
    name: String,
    query_text: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let conn = state.db.lock();
    saved::save(&conn, &name, &query_text, now).map_err(|e| e.to_string())
}

/// All saved searches, most recently created first.
#[tauri::command]
pub fn list_saved_searches(state: State<'_, AppState>) -> Result<Vec<SavedSearch>, String> {
    let conn = state.db.lock();
    saved::list(&conn).map_err(|e| e.to_string())
}

/// Delete a saved search by id.
#[tauri::command]
pub fn delete_saved_search(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock();
    saved::delete(&conn, id).map_err(|e| e.to_string())
}
