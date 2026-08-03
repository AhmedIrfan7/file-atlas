//! atlas-platform
//!
//! Isolates every operating-system difference behind the `PlatformFs` trait.
//! Core code depends on this trait, not on `cfg(target_os)` conditionals.
//! Each supported platform lives in its own module and implements the trait.
//!
//! ## Module map
//!
//! - `trait_defs` the `PlatformFs` trait and shared types
//! - `windows_impl` Windows implementation (Recycle Bin, Explorer, Shell APIs)
//! - `macos_impl` macOS implementation (Trash, Finder, `statfs`)
//! - `linux_impl` Linux implementation (freedesktop trash, `/proc/mounts`, `xdg-open`)
//! - `stub_impl` fallback for any other OS this project does not target

#![doc(html_root_url = "https://docs.rs/atlas-platform/0.1.0")]

pub mod trait_defs;
pub mod trash_common;

#[cfg(windows)]
pub mod windows_impl;

#[cfg(target_os = "macos")]
pub mod macos_impl;

#[cfg(target_os = "linux")]
pub mod linux_impl;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub mod stub_impl;

pub use trait_defs::{PlatformError, PlatformFs, Result, TrashHandle, Volume};

/// The concrete `PlatformFs` for the current OS.
#[cfg(windows)]
pub type CurrentFs = windows_impl::WindowsFs;

#[cfg(target_os = "macos")]
pub type CurrentFs = macos_impl::MacosFs;

#[cfg(target_os = "linux")]
pub type CurrentFs = linux_impl::LinuxFs;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub type CurrentFs = stub_impl::StubFs;

/// Convenience constructor.
#[must_use]
pub fn current() -> CurrentFs {
    CurrentFs::default()
}
