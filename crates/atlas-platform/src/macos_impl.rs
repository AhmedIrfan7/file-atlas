//! macOS implementation of `PlatformFs`.
//!
//! `libc::statfs` is the only FFI surface here (it gives block counts and the
//! filesystem type name in one call, which is why macOS uses it instead of
//! the POSIX `statvfs` that `linux_impl` uses). Device-id lookups reuse
//! `std::os::unix::fs::MetadataExt::dev()` instead of raw `libc::stat`, so
//! the only unsafe code in this file is the `statfs` call itself.

#![cfg(target_os = "macos")]
#![allow(unsafe_code)]

use std::ffi::CString;
use std::os::macos::fs::MetadataExt as _;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use crate::trait_defs::{PlatformError, PlatformFs, Result, TrashHandle, Volume};

/// `UF_HIDDEN` from `<sys/stat.h>`: the Finder "hide this item" flag, set via
/// `chflags hidden` independently of the dot-file naming convention.
const UF_HIDDEN: u32 = 0x0000_8000;

/// Paths under these prefixes are OS-owned. Deliberately excludes
/// `/Applications`: both Apple and the user install applications there, and
/// uninstalling an app by trashing it is ordinary user behavior, not a
/// system operation.
const SYSTEM_PREFIXES: &[&str] = &["/System", "/private", "/usr", "/bin", "/sbin", "/Library"];

#[derive(Debug, Default)]
pub struct MacosFs;

impl PlatformFs for MacosFs {
    fn send_to_trash(&self, path: &Path) -> Result<TrashHandle> {
        crate::trash_common::send_to_trash(path)
    }

    fn restore_from_trash(&self, handle: &TrashHandle) -> Result<PathBuf> {
        crate::trash_common::restore_from_trash(handle)
    }

    fn list_volumes(&self) -> Result<Vec<Volume>> {
        let root_dev = dev_id_for(Path::new("/"))?;
        let mut volumes = Vec::new();
        let mut root_included = false;

        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(dev) = dev_id_for(&path) else {
                    continue;
                };
                let is_root = dev == root_dev;
                if is_root {
                    root_included = true;
                }
                let (total, free, fs_type) = statfs_info(&path).unwrap_or((None, None, None));
                volumes.push(Volume {
                    id: format!("macos:{dev}"),
                    label: Some(entry.file_name().to_string_lossy().into_owned()),
                    fs_type,
                    mount: if is_root { PathBuf::from("/") } else { path },
                    total_bytes: total,
                    free_bytes: free,
                });
            }
        }

        if !root_included {
            let (total, free, fs_type) = statfs_info(Path::new("/")).unwrap_or((None, None, None));
            volumes.push(Volume {
                id: format!("macos:{root_dev}"),
                label: None,
                fs_type,
                mount: PathBuf::from("/"),
                total_bytes: total,
                free_bytes: free,
            });
        }

        Ok(volumes)
    }

    fn is_hidden(&self, path: &Path) -> Result<bool> {
        let starts_with_dot = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if starts_with_dot {
            return Ok(true);
        }
        let meta = std::fs::symlink_metadata(path)
            .map_err(|_| PlatformError::NotFound(path.to_path_buf()))?;
        Ok(meta.st_flags() & UF_HIDDEN != 0)
    }

    fn is_system(&self, path: &Path) -> Result<bool> {
        std::fs::symlink_metadata(path).map_err(|_| PlatformError::NotFound(path.to_path_buf()))?;
        let path_str = path.to_string_lossy();
        Ok(SYSTEM_PREFIXES
            .iter()
            .any(|prefix| path_str.starts_with(prefix)))
    }

    fn open_in_file_manager(&self, path: &Path) -> Result<()> {
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|e| PlatformError::Api(format!("failed to launch Finder: {e}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(PlatformError::Api(format!(
                "Finder reveal exited with {status}"
            )))
        }
    }
}

fn dev_id_for(path: &Path) -> Result<u64> {
    std::fs::metadata(path)
        .map(|m| m.dev())
        .map_err(|_| PlatformError::NotFound(path.to_path_buf()))
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn statfs_info(path: &Path) -> Option<(Option<u64>, Option<u64>, Option<String>)> {
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut buf: libc::statfs = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::statfs(c_path.as_ptr(), &raw mut buf) };
    if ret != 0 {
        return None;
    }
    let block_size = u64::from(buf.f_bsize);
    let total = block_size.checked_mul(buf.f_blocks as u64);
    let free = block_size.checked_mul(buf.f_bavail as u64);
    let fs_type_bytes: Vec<u8> = buf.f_fstypename.iter().map(|&c| c as u8).collect();
    let end = fs_type_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(fs_type_bytes.len());
    let fs_type = String::from_utf8_lossy(&fs_type_bytes[..end]).into_owned();
    Some((total, free, (!fs_type.is_empty()).then_some(fs_type)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_volumes_returns_at_least_the_root() {
        let fs = MacosFs;
        let vols = fs.list_volumes().expect("list volumes");
        assert!(!vols.is_empty(), "expected at least the root volume");
        assert!(vols.iter().any(|v| v.mount == Path::new("/")));
    }

    #[test]
    fn is_hidden_of_nonexistent_returns_notfound() {
        let fs = MacosFs;
        let err = fs.is_hidden(Path::new("/definitely_not_a_real_path_xyz_123"));
        assert!(matches!(err, Err(PlatformError::NotFound(_))));
    }

    #[test]
    fn dotfile_name_is_hidden_without_needing_the_file_to_exist() {
        // The dot-file check short-circuits before the UF_HIDDEN stat, so a
        // dotfile counts as hidden even if it never actually exists on disk.
        let fs = MacosFs;
        assert!(fs.is_hidden(Path::new("/tmp/.hidden_test_file")).unwrap());
    }

    #[test]
    fn plain_named_file_is_not_hidden() {
        let fs = MacosFs;
        let path = std::env::temp_dir().join("atlas_test_plain_file");
        std::fs::write(&path, b"x").unwrap();
        assert!(!fs.is_hidden(&path).unwrap());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn known_system_prefix_is_flagged() {
        let fs = MacosFs;
        assert!(fs.is_system(Path::new("/usr/bin/true")).unwrap());
    }

    #[test]
    fn applications_folder_is_not_flagged_as_system() {
        let fs = MacosFs;
        assert!(!fs.is_system(Path::new("/Applications")).unwrap());
    }
}
