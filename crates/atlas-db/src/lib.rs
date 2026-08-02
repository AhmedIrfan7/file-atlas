//! atlas-db
//!
//! Owns the SQLite index that backs everything File Atlas remembers about a
//! user's filesystem. Manages schema, migrations, connection lifecycle, and
//! typed queries. No business logic lives here. Consumers speak in terms of
//! `FileRow`, `VolumeRow`, `ActionRow`, and similar domain types.
//!
//! ## Module map
//!
//! - `connection` open + pragma configuration
//! - `migrations` embedded SQL migrations applied at startup
//! - `models` row types shared with `atlas-core`
//! - `queries` typed read and write helpers

#![doc(html_root_url = "https://docs.rs/atlas-db/0.1.0")]

pub mod connection;
pub mod migrations;
pub mod models;
pub mod queries;

pub use connection::{open, open_in_memory, DbError, Result};
pub use migrations::{apply as apply_migrations, Migration, MigrationError, MIGRATIONS};
pub use models::{ActionRow, FileRow, VolumeRow};
