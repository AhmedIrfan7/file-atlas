//! Shared trash/restore logic built on the `trash` crate.
//!
//! It wraps the real OS trash (Windows Recycle Bin, macOS Trash, the
//! freedesktop trash spec on Linux) instead of us hand-rolling platform
//! shell APIs for something this safety-critical.
//!
//! The `trash` crate's basic `delete()` does not hand back an identifier for
//! what it just moved. To make `restore_from_trash` possible on Windows and
//! Linux, we capture enough to re-find the item afterward: its original
//! parent folder, file name, and the deletion timestamp reported by the OS
//! trash listing (`trash::os_limited::list`). That triple is serialized
//! into `TrashHandle::token` as JSON.
//!
//! macOS is a partial exception: `trash::os_limited` is not available there
//! at all (checked directly against the crate: it is gated to Windows and to
//! "unix but not macOS/iOS/Android", i.e. the freedesktop backend), so there
//! is no crate-provided way to find a previously-trashed item again by
//! identity. `send_to_trash` still moves the file into the real macOS Trash
//! via `trash::delete`, so the operation is exactly as safe and exactly as
//! reversible by the user through Finder's own "Put Back" as everywhere
//! else; `restore_from_trash` reports `Unsupported` there rather than
//! guessing. See ADR 0010 for why a hand-rolled `NSFileManager` restore path
//! (which the underlying Cocoa API can support) was not attempted here: it
//! would mean shipping new, safety-critical Objective-C FFI with no way to
//! run it before it reaches a real Mac, since this project has no macOS
//! machine to test against.

use std::path::{Path, PathBuf};

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
#[cfg(not(target_os = "macos"))]
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

/// Send `path` to the real macOS Trash.
///
/// `os_limited` is not available on macOS (see module docs), so the
/// returned handle cannot yet be passed to a working `restore_from_trash`;
/// it still records enough to identify the item, ready for whenever that
/// gap is closed.
#[cfg(target_os = "macos")]
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

    let token = HandleToken {
        original_parent: parent.to_string_lossy().into_owned(),
        name: name.to_string_lossy().into_owned(),
        time_deleted: now_unix(),
    };
    Ok(TrashHandle {
        kind: HANDLE_KIND.to_string(),
        token: serde_json::to_string(&token)
            .map_err(|e| PlatformError::Api(format!("serialize trash handle: {e}")))?,
    })
}

/// Restore a file previously sent to trash via `send_to_trash`. Returns the
/// path it was restored to (its original location).
#[cfg(not(target_os = "macos"))]
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

/// Not supported on macOS: see module docs for why.
///
/// The item is still safely in the real Trash and restorable by the user
/// via Finder's own "Put Back"; this just means File Atlas's own restore
/// button cannot do it for them yet on this platform.
#[cfg(target_os = "macos")]
pub const fn restore_from_trash(_handle: &TrashHandle) -> Result<PathBuf> {
    Err(PlatformError::Unsupported)
}

#[cfg(not(target_os = "macos"))]
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
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // These tests touch the REAL OS Recycle Bin / Trash. They are marked
    // `ignore` so `cargo test` and CI stay side-effect-free; run explicitly
    // with `cargo test -p atlas-platform -- --ignored` to verify manually.

    #[cfg(not(target_os = "macos"))]
    #[test]
    #[ignore = "touches the real OS Recycle Bin / Trash"]
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

    #[cfg(not(target_os = "macos"))]
    #[test]
    #[ignore = "touches the real OS Recycle Bin / Trash"]
    fn restoring_wrong_kind_handle_is_an_error() {
        let bad_handle = TrashHandle {
            kind: "not-os-trash".to_string(),
            token: String::new(),
        };
        assert!(restore_from_trash(&bad_handle).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "touches the real macOS Trash"]
    fn send_to_trash_works_but_restore_reports_unsupported() {
        let dir = tempdir().expect("tempdir");
        let file_path = dir.path().join("atlas_trash_test.txt");
        fs::write(&file_path, b"file atlas trash test").expect("write scratch file");

        let handle = send_to_trash(&file_path).expect("send to trash");
        assert!(!file_path.exists(), "file should be gone after trashing");

        let err = restore_from_trash(&handle);
        assert!(matches!(err, Err(PlatformError::Unsupported)));
    }
}
