//! Tauri command handler for cleanup recommendations. Same thin-adapter
//! contract as `commands.rs`. Execution of a recommendation's items reuses
//! `duplicate_commands::trash_selected_paths` and `restore_trash_action`;
//! there is no separate execute path for recommendations.

use atlas_recommender::{get_recommendations, Recommendation};
use tauri::State;
use time::OffsetDateTime;

use crate::state::AppState;

/// Every current cleanup recommendation: empty folders, forgotten
/// installers, old archives, and screenshot pileups.
#[tauri::command]
pub fn get_cleanup_recommendations(
    state: State<'_, AppState>,
) -> Result<Vec<Recommendation>, String> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let conn = state.db.lock();
    get_recommendations(&conn, now).map_err(|e| e.to_string())
}
