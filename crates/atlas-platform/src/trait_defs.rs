//! The `PlatformFs` trait: the seam between core logic and per-OS behavior.
//!
//! Every method returns a typed `PlatformError` so callers can distinguish
//! "not supported on this platform" from "IO failed" from "permission denied".
//! Implementations live in the `windows`, `macos`, and `linux` modules.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A mounted drive or volume tracked by the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Volume {
    pub id: String,
    pub label: Option<String>,
    pub fs_type: Option<String>,
    pub mount: PathBuf,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
}

/// Opaque token returned by `send_to_trash` and passed back to `restore_from_trash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrashHandle {
    pub kind: String,
    pub token: String,
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("not supported on this platform")]
    Unsupported,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("platform api error: {0}")]
    Api(String),
    #[error("path not found: {0}")]
    NotFound(PathBuf),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// Filesystem-side capabilities that vary per operating system.
pub trait PlatformFs: Send + Sync + std::fmt::Debug {
    fn list_volumes(&self) -> Result<Vec<Volume>>;
    fn is_hidden(&self, path: &Path) -> Result<bool>;
    fn is_system(&self, path: &Path) -> Result<bool>;

    /// Send `path` to the OS Recycle Bin / Trash. Returns a handle usable for
    /// restore. Default implementation reports `Unsupported`; each platform
    /// module implements it for real.
    fn send_to_trash(&self, _path: &Path) -> Result<TrashHandle> {
        Err(PlatformError::Unsupported)
    }

    fn restore_from_trash(&self, _handle: &TrashHandle) -> Result<PathBuf> {
        Err(PlatformError::Unsupported)
    }

    fn open_in_file_manager(&self, _path: &Path) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
}
