// This crate is a Tauri app binary, not a library other crates depend on, so
// `pub` items that never leave the crate are expected rather than a smell.
#![allow(unreachable_pub)]
// Tauri's #[tauri::command] extractors (State, AppHandle, Window) must be
// taken by value; that is the framework's contract, not a design choice we
// can revisit per-command.
#![allow(clippy::needless_pass_by_value)]

mod commands;
mod state;
mod volume;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("resolve app data directory");
            std::fs::create_dir_all(&data_dir).expect("create app data directory");
            let db_path = data_dir.join("atlas.db");
            let state = AppState::new(&db_path).expect("initialize app state");
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_default_roots,
            commands::start_scan,
            commands::cancel_scan,
            commands::is_scanning,
            commands::get_home_summary,
            commands::get_top_largest,
            commands::get_top_oldest,
            commands::get_stale_bucket,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
