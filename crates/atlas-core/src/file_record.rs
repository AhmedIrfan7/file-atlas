//! The domain type emitted by the scanner and consumed by the indexer.
//!
//! A `FileRecord` is a snapshot of one filesystem entry at scan time. It
//! carries just enough information to persist into the SQLite index. It
//! never holds file contents.

use std::fs::Metadata;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use atlas_db::FileRow;

/// A snapshot of one filesystem entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: PathBuf,
    pub parent: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub accessed_at: Option<i64>,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub volume_id: String,
}

impl FileRecord {
    /// Build a `FileRecord` from a path plus stat metadata.
    pub fn from_metadata(
        path: &Path,
        meta: &Metadata,
        volume_id: impl Into<String>,
        is_hidden: bool,
    ) -> Self {
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let extension = path.extension().map(|e| e.to_string_lossy().into_owned());
        let parent = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        let created = meta.created().ok().map(system_time_to_unix);
        let modified = meta.modified().ok().map(system_time_to_unix);
        let accessed = meta.accessed().ok().map(system_time_to_unix);

        Self {
            path: path.to_path_buf(),
            parent,
            name,
            extension,
            size_bytes: if meta.is_dir() { 0 } else { meta.len() },
            created_at: created,
            modified_at: modified,
            accessed_at: accessed,
            is_dir: meta.is_dir(),
            is_hidden,
            is_symlink: meta.file_type().is_symlink(),
            volume_id: volume_id.into(),
        }
    }

    /// Convert to the database row shape. `first_seen` and `last_seen` are the
    /// current scan timestamp; the DB layer preserves `first_seen` across
    /// subsequent upserts of the same path.
    pub fn to_file_row(&self, scan_ts: i64, category: Option<String>) -> FileRow {
        FileRow {
            path: self.path.to_string_lossy().into_owned(),
            parent: self.parent.to_string_lossy().into_owned(),
            name: self.name.clone(),
            extension: self.extension.clone(),
            size_bytes: i64::try_from(self.size_bytes).unwrap_or(i64::MAX),
            created_at: self.created_at,
            modified_at: self.modified_at,
            accessed_at: self.accessed_at,
            hash_blake3: None,
            hash_size: None,
            category,
            is_dir: self.is_dir,
            is_hidden: self.is_hidden,
            is_symlink: self.is_symlink,
            volume_id: self.volume_id.clone(),
            first_seen: scan_ts,
            last_seen: scan_ts,
            removed_at: None,
        }
    }
}

fn system_time_to_unix(t: std::time::SystemTime) -> i64 {
    OffsetDateTime::from(t).unix_timestamp()
}
