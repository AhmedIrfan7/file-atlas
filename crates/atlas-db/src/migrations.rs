//! Embedded schema migrations.
//!
//! Each migration is a `(version, name, sql)` tuple embedded via `include_str!`
//! at compile time. Applied migrations are recorded in the `schema_migrations`
//! table. Running `apply` is idempotent: already-applied versions are skipped.
//!
//! Migrations are strictly forward-only. Rolling back a migration requires a
//! new forward migration that reverses it.

use rusqlite::{Connection, Transaction};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("sqlite error while applying migration {version:04} ({name}): {source}")]
    Apply {
        version: u32,
        name: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, MigrationError>;

/// A single migration. `sql` may contain multiple statements separated by `;`.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

/// The ordered list of migrations to apply. Append-only.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        sql: include_str!("../migrations/0001_initial_schema.sql"),
    },
    Migration {
        version: 2,
        name: "files_fts",
        sql: include_str!("../migrations/0002_fts.sql"),
    },
    Migration {
        version: 3,
        name: "saved_searches",
        sql: include_str!("../migrations/0003_saved_searches.sql"),
    },
];

/// Apply every pending migration to the given connection. Idempotent.
pub fn apply(conn: &mut Connection) -> Result<u32> {
    ensure_meta_table(conn)?;
    let applied = load_applied(conn)?;
    let mut count = 0u32;
    for migration in MIGRATIONS {
        if applied.contains(&migration.version) {
            continue;
        }
        run_one(conn, migration)?;
        count += 1;
    }
    Ok(count)
}

fn ensure_meta_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name    TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        );",
    )?;
    Ok(())
}

fn load_applied(conn: &Connection) -> Result<Vec<u32>> {
    let mut stmt = conn.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
    let rows = stmt
        .query_map([], |row| row.get::<_, u32>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn run_one(conn: &mut Connection, migration: &Migration) -> Result<()> {
    let tx: Transaction<'_> = conn.transaction()?;
    tx.execute_batch(migration.sql)
        .map_err(|source| MigrationError::Apply {
            version: migration.version,
            name: migration.name,
            source,
        })?;
    tx.execute(
        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            migration.version,
            migration.name,
            time::OffsetDateTime::now_utc().unix_timestamp(),
        ],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::open_in_memory;

    #[test]
    fn apply_on_fresh_db_returns_migration_count() {
        let mut conn = open_in_memory().expect("open in-memory");
        let count = apply(&mut conn).expect("apply migrations");
        assert_eq!(count as usize, MIGRATIONS.len());
    }

    #[test]
    fn apply_twice_is_idempotent() {
        let mut conn = open_in_memory().expect("open in-memory");
        apply(&mut conn).expect("apply first");
        let second = apply(&mut conn).expect("apply second");
        assert_eq!(second, 0);
    }
}
