//! Shared trash/restore logic built on the `trash` crate.
//!
//! It wraps the real OS trash (Windows Recycle Bin, macOS Trash, the
//! freedesktop trash spec on Linux) instead of us hand-rolling platform
//! shell APIs for something this safety-critical.
//!
//! The `trash` crate's basic `delete()` does not hand back an identifier for
//! what it just moved. To make `restore_from_trash` possible, we capture
//! enough to re-find the item afterward: its original parent folder, file
//! name, and the deletion timestamp reported by the OS trash listing. That
//! triple is serialized into `TrashHandle::token` as JSON.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::trait_defs::{PlatformError, Result, TrashHandle};

const HANDLE_KIND: &str = "os-trash";

#[derive(Debug, Serialize, Deserialize)]
struct HandleToken {
    original_parent: String,
    name: String,
    time_deleted: i64,
}

/// Send `path` to the OS trash and return a handle that can later be passed
/// to `restore_from_trash`.
pub fn send_to_trash(path: &Path) -> Result<TrashHandle> {
    if !path.exists() {
        return Err(PlatformError::NotFound(path.to_path_buf()));
    }
    let parent = path
        .parent()
        .map_or_else(|| PathBuf::from(""), Path::to_path_buf);
    let name = path
        .file_name()
        .ok_or_else(|| PlatformError::Api(format!("path has no file name: {}", path.display())))?
        .to_owned();

    trash::delete(path).map_err(|e| PlatformError::Api(format!("send to trash failed: {e}")))?;

    let deleted_after = now_unix();
    let item = find_matching_item(&parent, &name.to_string_lossy(), deleted_after)?;

    let token = HandleToken {
        original_parent: parent.to_string_lossy().into_owned(),
        name: name.to_string_lossy().into_owned(),
        time_deleted: item.time_deleted,
    };
    Ok(TrashHandle {
        kind: HANDLE_KIND.to_string(),
        token: serde_json::to_string(&token)
            .map_err(|e| PlatformError::Api(format!("serialize trash handle: {e}")))?,
    })
}

/// Restore a file previously sent to trash via `send_to_trash`. Returns the
/// path it was restored to (its original location).
pub fn restore_from_trash(handle: &TrashHandle) -> Result<PathBuf> {
    if handle.kind != HANDLE_KIND {
        return Err(PlatformError::Api(format!(
            "unrecognized trash handle kind: {}",
            handle.kind
        )));
    }
    let token: HandleToken = serde_json::from_str(&handle.token)
        .map_err(|e| PlatformError::Api(format!("deserialize trash handle: {e}")))?;

    let parent = PathBuf::from(&token.original_parent);
    let all_items = trash::os_limited::list()
        .map_err(|e| PlatformError::Api(format!("list trash failed: {e}")))?;
    let item = all_items
        .into_iter()
        .find(|i| {
            i.name.to_string_lossy() == token.name
                && i.original_parent == parent
                && i.time_deleted == token.time_deleted
        })
        .ok_or_else(|| PlatformError::NotFound(parent.join(&token.name)))?;

    let restored_path = parent.join(&token.name);
    trash::os_limited::restore_all([item])
        .map_err(|e| PlatformError::Api(format!("restore from trash failed: {e}")))?;
    Ok(restored_path)
}

fn find_matching_item(parent: &Path, name: &str, not_after: i64) -> Result<trash::TrashItem> {
    let items = trash::os_limited::list()
        .map_err(|e| PlatformError::Api(format!("list trash failed: {e}")))?;
    items
        .into_iter()
        .filter(|i| {
            i.name.to_string_lossy() == name
                && i.original_parent == parent
                && i.time_deleted <= not_after
        })
        .max_by_key(|i| i.time_deleted)
        .ok_or_else(|| {
            PlatformError::Api(format!(
                "could not find {} in trash after deleting it",
                parent.join(name).display()
            ))
        })
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // These tests touch the REAL OS Recycle Bin. They are marked `ignore` so
    // `cargo test` and CI stay side-effect-free; run explicitly with
    // `cargo test -p atlas-platform -- --ignored` to verify manually.

    #[test]
    #[ignore = "touches the real OS Recycle Bin"]
    fn send_then_restore_roundtrips() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("atlas_trash_test.txt");
        fs::write(&file_path, b"file atlas trash test").expect("write scratch file");

        let handle = send_to_trash(&file_path).expect("send to trash");
        assert!(!file_path.exists(), "file should be gone after trashing");

        let restored = restore_from_trash(&handle).expect("restore from trash");
        assert_eq!(restored, file_path);
        assert!(file_path.exists(), "file should be back after restore");

        let contents = fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents, "file atlas trash test");

        fs::remove_file(&file_path).ok();
    }

    #[test]
    #[ignore = "touches the real OS Recycle Bin"]
    fn restoring_wrong_kind_handle_is_an_error() {
        let bad_handle = TrashHandle {
            kind: "not-os-trash".to_string(),
            token: String::new(),
        };
        assert!(restore_from_trash(&bad_handle).is_err());
    }
}
