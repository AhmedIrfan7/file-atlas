//! Tauri command handlers for the life timeline (M7): a day/month histogram
//! of file creation, plus auto-detected screenshot and project bursts. Same
//! thin-adapter contract as `storage_commands.rs`: pure reads, no execution
//! path of their own.

use atlas_core::{
    get_timeline, project_bursts, screenshot_bursts, Burst, Granularity, TimelineResponse,
    BURST_SAMPLE_LIMIT, PROJECT_BURST_MIN_COUNT, SCREENSHOT_BURST_MIN_COUNT,
};
use tauri::State;
use time::OffsetDateTime;

use crate::state::AppState;

const SECONDS_PER_DAY: i64 = 86_400;

/// The timeline histogram at `granularity`, optionally restricted to the
/// last `since_days` days.
#[tauri::command]
pub fn get_life_timeline(
    granularity: Granularity,
    since_days: Option<u32>,
    state: State<'_, AppState>,
) -> Result<TimelineResponse, String> {
    let since_unix = since_days
        .map(|days| OffsetDateTime::now_utc().unix_timestamp() - i64::from(days) * SECONDS_PER_DAY);
    let conn = state.db.lock();
    get_timeline(&conn, granularity, since_unix).map_err(|e| e.to_string())
}

/// Days with an unusually high number of screenshot creations, optionally
/// restricted to the last `since_days` days. Thresholds are fixed (see
/// `atlas_core::timeline`), not user-configurable yet.
///
/// `since_days` matching `get_life_timeline`'s parameter is deliberate: both
/// are driven by the same view-window selector in the UI, and previously
/// only the histogram actually respected it while bursts silently always
/// showed all-time data regardless of the selected window.
#[tauri::command]
pub fn get_screenshot_bursts(
    since_days: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<Burst>, String> {
    let since_unix = since_days
        .map(|days| OffsetDateTime::now_utc().unix_timestamp() - i64::from(days) * SECONDS_PER_DAY);
    let conn = state.db.lock();
    screenshot_bursts(
        &conn,
        SCREENSHOT_BURST_MIN_COUNT,
        BURST_SAMPLE_LIMIT,
        since_unix,
    )
    .map_err(|e| e.to_string())
}

/// Folder-and-day pairs with an unusually high number of file creations at
/// once, optionally restricted to the last `since_days` days. Thresholds are
/// fixed (see `atlas_core::timeline`), not user-configurable yet.
#[tauri::command]
pub fn get_project_bursts(
    since_days: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<Burst>, String> {
    let since_unix = since_days
        .map(|days| OffsetDateTime::now_utc().unix_timestamp() - i64::from(days) * SECONDS_PER_DAY);
    let conn = state.db.lock();
    project_bursts(
        &conn,
        PROJECT_BURST_MIN_COUNT,
        BURST_SAMPLE_LIMIT,
        since_unix,
    )
    .map_err(|e| e.to_string())
}
