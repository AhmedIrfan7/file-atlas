//! atlas-platform
//!
//! Isolates every operating-system difference behind the `PlatformFs` trait.
//! Core code depends on this trait, not on `cfg(target_os)` conditionals.
//! Each supported platform lives in its own module and implements the trait.
//!
//! ## Module map
//!
//! - `trait_defs` the `PlatformFs` trait and shared types
//! - `windows_impl` Windows implementation (Recycle Bin, Shell APIs)
//! - `stub_impl` non-Windows fallback until M8 fills in macOS and Linux

#![doc(html_root_url = "https://docs.rs/atlas-platform/0.1.0")]

pub mod trait_defs;
pub mod trash_common;

#[cfg(windows)]
pub mod windows_impl;

#[cfg(not(windows))]
pub mod stub_impl;

pub use trait_defs::{PlatformError, PlatformFs, Result, TrashHandle, Volume};

/// The concrete `PlatformFs` for the current OS.
#[cfg(windows)]
pub type CurrentFs = windows_impl::WindowsFs;

#[cfg(not(windows))]
pub type CurrentFs = stub_impl::StubFs;

/// Convenience constructor.
#[must_use]
pub fn current() -> CurrentFs {
    CurrentFs::default()
}
