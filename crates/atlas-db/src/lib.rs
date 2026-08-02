//! atlas-db
//!
//! Owns the SQLite index that backs everything File Atlas remembers about a
//! user's filesystem. Manages schema, migrations, connection lifecycle, and
//! typed queries. No business logic lives here. Consumers speak in terms of
//! `FileRecord`, `Volume`, `ActionLogEntry`, and similar domain types.
//!
//! ## Module map (populated across milestones)
//!
//! - `schema` DDL and migrations
//! - `connection` pool and WAL configuration
//! - `queries` typed read and write helpers
//! - `models` row types shared with `atlas-core`

#![doc(html_root_url = "https://docs.rs/atlas-db/0.1.0")]
