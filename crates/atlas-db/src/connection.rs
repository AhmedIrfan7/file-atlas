//! SQLite connection setup for the File Atlas index.
//!
//! The index is single-writer / many-reader. Callers get a `Connection`
//! configured with WAL journaling and sensible pragmas. Concurrent readers
//! are safe; concurrent writers are not, and the higher-level `atlas-core`
//! serializes writes through a single writer task (see ADR 0003).

use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("open failed for {path}: {source}")]
    Open {
        path: String,
        #[source]
        source: rusqlite::Error,
    },
    #[error("pragma configuration failed: {0}")]
    Pragma(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Open the atlas index at `path`. Creates the file if missing. Applies WAL
/// mode and the pragmas the rest of the codebase assumes.
pub fn open(path: impl AsRef<Path>) -> Result<Connection> {
    let path_ref = path.as_ref();
    let conn = Connection::open_with_flags(
        path_ref,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|source| DbError::Open {
        path: path_ref.display().to_string(),
        source,
    })?;
    configure(&conn)?;
    Ok(conn)
}

/// Open an in-memory database. Useful for tests.
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory().map_err(|source| DbError::Open {
        path: ":memory:".into(),
        source,
    })?;
    configure(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "cache_size", -65_536_i64)?;
    Ok(())
}
