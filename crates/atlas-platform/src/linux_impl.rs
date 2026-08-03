//! Linux implementation of `PlatformFs`.
//!
//! Volumes come from parsing `/proc/mounts` rather than `libudev` or a
//! similar heavier dependency: it is a plain-text table the kernel already
//! maintains, present on every Linux system regardless of init system or
//! desktop environment. Pseudo/virtual filesystems (`proc`, `sysfs`,
//! `tmpfs`, container overlays, and similar) are filtered out by filesystem
//! type so they never show up as "volumes" a user would recognize.
//!
//! `libc::statvfs` is the only FFI surface here, used purely for the total
//! and free byte counts of each real mount point.

#![cfg(target_os = "linux")]
#![allow(unsafe_code)]

use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::trait_defs::{PlatformError, PlatformFs, Result, TrashHandle, Volume};

/// Filesystem types that are kernel/container bookkeeping, not real storage
/// a user would think of as "a volume".
static VIRTUAL_FS_TYPES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "tmpfs",
        "cgroup",
        "cgroup2",
        "pstore",
        "bpf",
        "tracefs",
        "debugfs",
        "mqueue",
        "hugetlbfs",
        "fusectl",
        "configfs",
        "securityfs",
        "autofs",
        "binfmt_misc",
        "rpc_pipefs",
        "nsfs",
        "overlay",
        "squashfs",
        "efivarfs",
        "ramfs",
    ]
    .into_iter()
    .collect()
});

/// Paths under these prefixes are OS-owned. Deliberately excludes `/opt` and
/// `/home`: `/opt` is where third-party applications install, and removing
/// one is ordinary user behavior, not a system operation.
const SYSTEM_PREFIXES: &[&str] = &[
    "/proc", "/sys", "/dev", "/boot", "/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/run",
];

#[derive(Debug, Default)]
pub struct LinuxFs;

impl PlatformFs for LinuxFs {
    fn send_to_trash(&self, path: &Path) -> Result<TrashHandle> {
        crate::trash_common::send_to_trash(path)
    }

    fn restore_from_trash(&self, handle: &TrashHandle) -> Result<PathBuf> {
        crate::trash_common::restore_from_trash(handle)
    }

    fn list_volumes(&self) -> Result<Vec<Volume>> {
        let contents = fs::read_to_string("/proc/mounts")
            .map_err(|e| PlatformError::Api(format!("read /proc/mounts: {e}")))?;

        let mut volumes = Vec::new();
        for line in contents.lines() {
            let mut fields = line.split_whitespace();
            let (Some(device), Some(mountpoint), Some(fs_type)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if device == "none" || VIRTUAL_FS_TYPES.contains(fs_type) {
                continue;
            }
            let mount = unescape_octal(mountpoint);
            let (total, free) = statvfs_bytes(&mount).unwrap_or((None, None));
            volumes.push(Volume {
                id: format!("linux:{}", mount.display()),
                label: mount.file_name().map(|n| n.to_string_lossy().into_owned()),
                fs_type: Some(fs_type.to_string()),
                mount,
                total_bytes: total,
                free_bytes: free,
            });
        }
        Ok(volumes)
    }

    fn is_hidden(&self, path: &Path) -> Result<bool> {
        // Standard Linux desktop convention: a leading dot means hidden.
        // There is no separate filesystem attribute bit to check the way
        // Windows and macOS have; ADR 0010 covers what this omits.
        fs::symlink_metadata(path).map_err(|_| PlatformError::NotFound(path.to_path_buf()))?;
        Ok(path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.')))
    }

    fn is_system(&self, path: &Path) -> Result<bool> {
        fs::symlink_metadata(path).map_err(|_| PlatformError::NotFound(path.to_path_buf()))?;
        let path_str = path.to_string_lossy();
        Ok(SYSTEM_PREFIXES
            .iter()
            .any(|prefix| path_str.starts_with(prefix)))
    }

    fn open_in_file_manager(&self, path: &Path) -> Result<()> {
        // There is no universal "reveal and select this exact file" verb
        // across Linux file managers the way Explorer's `/select,` or
        // Finder's `open -R` work; opening the containing folder is the
        // portable substitute every `xdg-open`-compatible file manager
        // understands.
        let target = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        let status = std::process::Command::new("xdg-open")
            .arg(target)
            .status()
            .map_err(|e| PlatformError::Api(format!("failed to launch xdg-open: {e}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(PlatformError::Api(format!("xdg-open exited with {status}")))
        }
    }
}

/// `/proc/mounts` escapes spaces, tabs, backslashes, and newlines in paths as
/// octal sequences (e.g. `\040` for a space). Reverse that so the mount path
/// matches what actually exists on disk.
fn unescape_octal(s: &str) -> PathBuf {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            if let Ok(code) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 4]).unwrap_or_default(),
                8,
            ) {
                out.push(code);
                i += 4;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    PathBuf::from(std::ffi::OsStr::from_bytes(&out))
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn statvfs_bytes(path: &Path) -> Option<(Option<u64>, Option<u64>)> {
    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut buf: libc::statvfs = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::statvfs(c_path.as_ptr(), &raw mut buf) };
    if ret != 0 {
        return None;
    }
    let block_size = buf.f_frsize as u64;
    let total = block_size.checked_mul(buf.f_blocks as u64);
    let free = block_size.checked_mul(buf.f_bavail as u64);
    Some((total, free))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_volumes_returns_at_least_the_root() {
        let fs = LinuxFs;
        let vols = fs.list_volumes().expect("list volumes");
        assert!(
            !vols.is_empty(),
            "expected at least one real mount, including root"
        );
        assert!(vols.iter().any(|v| v.mount == Path::new("/")));
    }

    #[test]
    fn list_volumes_excludes_virtual_filesystems() {
        let fs = LinuxFs;
        let vols = fs.list_volumes().expect("list volumes");
        assert!(
            !vols.iter().any(|v| v.fs_type.as_deref() == Some("proc")),
            "proc should never be reported as a volume"
        );
    }

    #[test]
    fn is_hidden_of_nonexistent_returns_notfound() {
        let fs = LinuxFs;
        let err = fs.is_hidden(Path::new("/definitely_not_a_real_path_xyz_123"));
        assert!(matches!(err, Err(PlatformError::NotFound(_))));
    }

    #[test]
    fn dotfile_name_is_hidden() {
        let fs = LinuxFs;
        let dir = std::env::temp_dir();
        let hidden = dir.join(".atlas_test_hidden_file");
        let plain = dir.join("atlas_test_plain_file");
        std::fs::write(&hidden, b"x").unwrap();
        std::fs::write(&plain, b"x").unwrap();

        assert!(fs.is_hidden(&hidden).unwrap());
        assert!(!fs.is_hidden(&plain).unwrap());

        std::fs::remove_file(&hidden).ok();
        std::fs::remove_file(&plain).ok();
    }

    #[test]
    fn known_system_prefix_is_flagged() {
        let fs = LinuxFs;
        assert!(fs.is_system(Path::new("/usr/bin/true")).unwrap());
    }

    #[test]
    fn opt_is_not_flagged_as_system() {
        let fs = LinuxFs;
        // /opt may not exist on every distro; only assert when it does, since
        // is_system errors NotFound for a path that is not there.
        if Path::new("/opt").exists() {
            assert!(!fs.is_system(Path::new("/opt")).unwrap());
        }
    }

    #[test]
    fn unescape_octal_reverses_proc_mounts_space_escaping() {
        assert_eq!(
            unescape_octal("/mnt/My\\040Drive"),
            PathBuf::from("/mnt/My Drive")
        );
        assert_eq!(unescape_octal("/plain/path"), PathBuf::from("/plain/path"));
    }
}
