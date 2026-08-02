//! atlas-platform
//!
//! Isolates every operating-system difference behind a `PlatformFs` trait.
//! Core code depends on this trait, not on `cfg(target_os)` conditionals.
//! Each supported platform lives in its own module and implements the trait.
//!
//! ## Module map (populated across milestones)
//!
//! - `trait_defs` the `PlatformFs` trait and shared types
//! - `windows` Windows implementation (Recycle Bin via IFileOperation, Shell APIs)
//! - `macos` macOS implementation (NSFileManager trashItemAtURL)
//! - `linux` Linux implementation (freedesktop trash spec via the `trash` crate)

#![doc(html_root_url = "https://docs.rs/atlas-platform/0.1.0")]
