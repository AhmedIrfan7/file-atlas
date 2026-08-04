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
    pub hash_cancel: Arc<AtomicBool>,
    pub hash_running: Arc<AtomicBool>,
    pub embed_cancel: Arc<AtomicBool>,
    pub embed_running: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(db_path: &Path) -> anyhow::Result<Self> {
        let mut conn = open(db_path)?;
        apply_migrations(&mut conn)?;
        atlas_core::seed_protected_paths(&conn, now_unix())?;
        Ok(Self {
            db: Mutex::new(conn),
            scan_cancel: Arc::new(AtomicBool::new(false)),
            scan_running: Arc::new(AtomicBool::new(false)),
            hash_cancel: Arc::new(AtomicBool::new(false)),
            hash_running: Arc::new(AtomicBool::new(false)),
            embed_cancel: Arc::new(AtomicBool::new(false)),
            embed_running: Arc::new(AtomicBool::new(false)),
        })
    }
}

fn now_unix() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
