//! Suggested scan roots for first-run onboarding.
//!
//! File Atlas never scans the whole drive on first run. Instead it proposes
//! the handful of folders where clutter actually accumulates: Desktop,
//! Downloads, Documents, Pictures, Videos, Music. The user can accept some,
//! all, or none, and can add custom roots from the onboarding wizard.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A single suggested root with a human-friendly label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestedRoot {
    pub label: String,
    pub path: PathBuf,
}

/// Known user folders that exist on this machine, in a sensible display order.
///
/// Folders that could not be resolved (e.g. redirected away, or not present
/// on this platform) are omitted rather than guessed at.
#[must_use]
pub fn default_roots() -> Vec<SuggestedRoot> {
    let candidates: [(&str, Option<PathBuf>); 6] = [
        ("Desktop", dirs::desktop_dir()),
        ("Downloads", dirs::download_dir()),
        ("Documents", dirs::document_dir()),
        ("Pictures", dirs::picture_dir()),
        ("Videos", dirs::video_dir()),
        ("Music", dirs::audio_dir()),
    ];

    candidates
        .into_iter()
        .filter_map(|(label, path)| {
            path.filter(|p| p.exists()).map(|path| SuggestedRoot {
                label: label.to_string(),
                path,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_roots_only_returns_existing_paths() {
        for root in default_roots() {
            assert!(root.path.exists(), "{} should exist", root.path.display());
        }
    }
}
