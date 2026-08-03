//! Fallback stub for any OS this project does not target (Windows, macOS,
//! and Linux all have real implementations as of M8). Enough to compile;
//! every method reports `Unsupported`.

#![cfg(not(any(windows, target_os = "macos", target_os = "linux")))]

use std::path::Path;

use crate::trait_defs::{PlatformError, PlatformFs, Result, Volume};

#[derive(Debug, Default)]
pub struct StubFs;

impl PlatformFs for StubFs {
    fn list_volumes(&self) -> Result<Vec<Volume>> {
        Err(PlatformError::Unsupported)
    }

    fn is_hidden(&self, _path: &Path) -> Result<bool> {
        Err(PlatformError::Unsupported)
    }

    fn is_system(&self, _path: &Path) -> Result<bool> {
        Err(PlatformError::Unsupported)
    }
}
