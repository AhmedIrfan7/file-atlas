//! Row types shared with `atlas-core`.
//!
//! These are plain data. They deliberately expose primitive types (integers
//! for timestamps, `String` for paths) so the SQL layer stays boring. Higher
//! layers wrap these in richer domain types.

use serde::{Deserialize, Serialize};

/// A single filesystem entry as stored in the `files` table. Directories are
/// represented with `is_dir = true` and `size_bytes = 0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRow {
    pub path: String,
    pub parent: String,
    pub name: String,
    pub extension: Option<String>,
    pub size_bytes: i64,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub accessed_at: Option<i64>,
    pub hash_blake3: Option<String>,
    pub hash_size: Option<i64>,
    pub category: Option<String>,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub volume_id: String,
    pub first_seen: i64,
    pub last_seen: i64,
    pub removed_at: Option<i64>,
}

/// Metadata about a volume (drive) tracked by the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeRow {
    pub id: String,
    pub label: Option<String>,
    pub fs_type: Option<String>,
    pub mount: String,
    pub total_bytes: Option<i64>,
    pub first_seen: i64,
    pub last_seen: i64,
}

/// One row from `actions_log`. Used to render undo history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRow {
    pub id: i64,
    pub ts: i64,
    pub op: String,
    pub path_from: Option<String>,
    pub path_to: Option<String>,
    pub metadata: Option<String>,
    pub reversible: bool,
    pub undo_ref: Option<String>,
}
