//! Shared application state managed by Tauri.
//!
//! One SQLite connection lives behind a mutex for the lifetime of the app.
//! This matches the single-writer model from ADR 0003: every command that
//! touches the database goes through this one connection, so writes and
//! reads never race each other.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use atlas_db::{apply_migrations, open};
use parking_lot::Mutex;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub scan_cancel: Arc<AtomicBool>,
    pub scan_running: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(db_path: &Path) -> anyhow::Result<Self> {
        let mut conn = open(db_path)?;
        apply_migrations(&mut conn)?;
        Ok(Self {
            db: Mutex::new(conn),
            scan_cancel: Arc::new(AtomicBool::new(false)),
            scan_running: Arc::new(AtomicBool::new(false)),
        })
    }
}
