//! Small helper for mapping a scan root to the volume that contains it.
//!
//! Duplicated (intentionally, it is a handful of lines) from the same logic
//! in `apps/cli/src/main.rs`. Both call sites need "which mounted volume
//! contains this path" and neither needs a shared crate for it yet.

use std::path::Path;

use atlas_db::VolumeRow;
use atlas_platform::{current, PlatformFs, Volume};
use time::OffsetDateTime;

pub fn resolve_volume_for(path: &Path) -> Volume {
    let plat = current();
    if let Ok(vols) = plat.list_volumes() {
        let path_str = path.to_string_lossy();
        let mut best: Option<Volume> = None;
        for v in vols {
            let mount_str = v.mount.to_string_lossy();
            let trimmed = mount_str.trim_end_matches(['\\', '/']);
            if starts_with_ci(&path_str, trimmed) {
                let longer = best
                    .as_ref()
                    .is_none_or(|b| b.mount.as_os_str().len() < v.mount.as_os_str().len());
                if longer {
                    best = Some(v);
                }
            }
        }
        if let Some(v) = best {
            return v;
        }
    }
    Volume {
        id: "vol:unknown".to_string(),
        label: None,
        fs_type: None,
        mount: path.to_path_buf(),
        total_bytes: None,
        free_bytes: None,
    }
}

pub fn to_volume_row(v: &Volume) -> VolumeRow {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    VolumeRow {
        id: v.id.clone(),
        label: v.label.clone(),
        fs_type: v.fs_type.clone(),
        mount: v.mount.to_string_lossy().into_owned(),
        total_bytes: v.total_bytes.and_then(|n| i64::try_from(n).ok()),
        first_seen: now,
        last_seen: now,
    }
}

fn starts_with_ci(haystack: &str, needle: &str) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    haystack
        .chars()
        .zip(needle.chars())
        .all(|(a, b)| a.eq_ignore_ascii_case(&b))
}
