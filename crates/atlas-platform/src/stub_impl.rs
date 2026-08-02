//! Non-Windows stub implementation. Enough to compile and to run tests that
//! do not actually touch the filesystem, so macOS and Linux CI stays green
//! until real implementations arrive in M8.

#![cfg(not(windows))]

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
