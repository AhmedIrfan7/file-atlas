//! Windows implementation of `PlatformFs`.
//!
//! FFI calls to the Windows API are the only unsafe code in the workspace.
//! Each unsafe block is small and takes only stack-allocated buffers.

#![cfg(windows)]
#![allow(unsafe_code)]

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetFileAttributesW, GetLogicalDrives,
    GetVolumeInformationW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM, INVALID_FILE_ATTRIBUTES,
};

use crate::trait_defs::{PlatformError, PlatformFs, Result, TrashHandle, Volume};

#[derive(Debug, Default)]
pub struct WindowsFs;

impl PlatformFs for WindowsFs {
    fn send_to_trash(&self, path: &Path) -> Result<TrashHandle> {
        crate::trash_common::send_to_trash(path)
    }

    fn restore_from_trash(&self, handle: &TrashHandle) -> Result<PathBuf> {
        crate::trash_common::restore_from_trash(handle)
    }

    fn list_volumes(&self) -> Result<Vec<Volume>> {
        let mask = unsafe { GetLogicalDrives() };
        if mask == 0 {
            return Err(PlatformError::Api("GetLogicalDrives returned 0".into()));
        }

        let mut volumes = Vec::new();
        for i in 0..26u32 {
            if mask & (1 << i) == 0 {
                continue;
            }
            let letter = char::from_u32(u32::from(b'A') + i).unwrap_or('?');
            let mount = format!("{letter}:\\");
            match volume_for_root(&mount) {
                Ok(v) => volumes.push(v),
                Err(err) => tracing::debug!(mount = %mount, error = %err, "skip volume"),
            }
        }
        Ok(volumes)
    }

    fn is_hidden(&self, path: &Path) -> Result<bool> {
        let attrs = get_attrs(path)?;
        Ok(attrs & FILE_ATTRIBUTE_HIDDEN.0 != 0)
    }

    fn is_system(&self, path: &Path) -> Result<bool> {
        let attrs = get_attrs(path)?;
        Ok(attrs & FILE_ATTRIBUTE_SYSTEM.0 != 0)
    }

    fn open_in_file_manager(&self, path: &Path) -> Result<()> {
        // explorer.exe parses its own command line rather than using
        // standard argv splitting, so `/select,<path>` must be one argument,
        // not `/select,` and the path as two separate ones.
        let select_arg = format!("/select,{}", path.display());
        std::process::Command::new("explorer.exe")
            .arg(select_arg)
            .spawn()
            .map_err(|e| PlatformError::Api(format!("failed to launch Explorer: {e}")))?;
        Ok(())
    }
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn get_attrs(path: &Path) -> Result<u32> {
    let wide = to_wide_null(&path.to_string_lossy());
    let attrs = unsafe { GetFileAttributesW(windows::core::PCWSTR(wide.as_ptr())) };
    if attrs == INVALID_FILE_ATTRIBUTES {
        return Err(PlatformError::NotFound(path.to_path_buf()));
    }
    Ok(attrs)
}

fn volume_for_root(mount: &str) -> Result<Volume> {
    let root = to_wide_null(mount);
    let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(root.as_ptr())) };
    // 1 = DRIVE_NO_ROOT_DIR
    if drive_type == 1 {
        return Err(PlatformError::NotFound(PathBuf::from(mount)));
    }

    let mut label = vec![0u16; MAX_PATH as usize];
    let mut fs = vec![0u16; MAX_PATH as usize];
    let mut serial: u32 = 0;
    let mut max_component: u32 = 0;
    let mut flags: u32 = 0;
    let ok = unsafe {
        GetVolumeInformationW(
            windows::core::PCWSTR(root.as_ptr()),
            Some(&mut label),
            Some(&raw mut serial),
            Some(&raw mut max_component),
            Some(&raw mut flags),
            Some(&mut fs),
        )
    };

    let (label_str, fs_str) = if ok.is_ok() {
        (Some(wide_to_string(&label)), Some(wide_to_string(&fs)))
    } else {
        (None, None)
    };

    let (total, free) = free_bytes_for(mount).unwrap_or((None, None));

    Ok(Volume {
        id: format!("win:{}:{serial:08x}", mount.chars().next().unwrap_or('?')),
        label: label_str.filter(|s| !s.is_empty()),
        fs_type: fs_str.filter(|s| !s.is_empty()),
        mount: PathBuf::from(mount),
        total_bytes: total,
        free_bytes: free,
    })
}

fn free_bytes_for(mount: &str) -> Option<(Option<u64>, Option<u64>)> {
    let root = to_wide_null(mount);
    let mut free_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut free_total: u64 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            windows::core::PCWSTR(root.as_ptr()),
            Some(&raw mut free_caller),
            Some(&raw mut total),
            Some(&raw mut free_total),
        )
    };
    if ok.is_err() {
        return None;
    }
    Some((Some(total), Some(free_total)))
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_volumes_returns_at_least_one_drive() {
        let fs = WindowsFs;
        let vols = fs.list_volumes().expect("list volumes");
        assert!(!vols.is_empty(), "expected at least one mounted volume");
        assert!(vols.iter().all(|v| v.id.starts_with("win:")));
    }

    #[test]
    fn is_hidden_of_nonexistent_returns_notfound() {
        let fs = WindowsFs;
        let err = fs.is_hidden(Path::new("C:\\definitely_not_a_real_path_xyz_123"));
        assert!(matches!(err, Err(PlatformError::NotFound(_))));
    }
}
