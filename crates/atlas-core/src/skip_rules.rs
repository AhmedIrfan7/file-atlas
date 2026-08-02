//! Rules for what the scanner should NOT descend into.
//!
//! Skip rules apply BEFORE indexing. They exist to save time and to avoid
//! entering directories that are known to be either irrelevant (build caches)
//! or dangerous to touch (Windows system paths). Users can opt in to hidden
//! files and can add custom directory names to skip.
//!
//! The intent is that a fresh install of File Atlas does a fast, safe walk of
//! a user's data directories out of the box. Deep or exotic locations require
//! an explicit opt-in.

use std::collections::HashSet;
use std::path::Path;

/// Configuration for what the scanner ignores.
#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct SkipRules {
    /// Directory names whose subtrees are skipped entirely.
    pub skip_dir_names: HashSet<String>,
    /// Whole path prefixes to skip (case-insensitive on Windows).
    pub skip_path_prefixes: Vec<String>,
    /// If true, hidden files and directories are skipped.
    pub skip_hidden: bool,
    /// If true, symbolic links are not followed. Recommended.
    pub skip_symlinks: bool,
}

impl Default for SkipRules {
    fn default() -> Self {
        let mut skip_dir_names: HashSet<String> = HashSet::new();
        for name in [
            ".git",
            ".hg",
            ".svn",
            "node_modules",
            "__pycache__",
            ".venv",
            ".mypy_cache",
            ".pytest_cache",
            ".tox",
            "target",
            "build",
            "dist",
            ".next",
            ".nuxt",
            ".turbo",
            ".cache",
            ".gradle",
            ".idea",
            ".vscode",
            "$RECYCLE.BIN",
            "System Volume Information",
        ] {
            skip_dir_names.insert(name.to_string());
        }

        let skip_path_prefixes = vec![
            "C:\\Windows".into(),
            "C:\\Program Files".into(),
            "C:\\Program Files (x86)".into(),
            "C:\\ProgramData".into(),
            "C:\\$Recycle.Bin".into(),
            "C:\\System Volume Information".into(),
        ];

        Self {
            skip_dir_names,
            skip_path_prefixes,
            skip_hidden: true,
            skip_symlinks: true,
        }
    }
}

impl SkipRules {
    /// Empty ruleset. Everything is walked. Useful for tests.
    #[must_use]
    pub fn permissive() -> Self {
        Self {
            skip_dir_names: HashSet::new(),
            skip_path_prefixes: Vec::new(),
            skip_hidden: false,
            skip_symlinks: false,
        }
    }

    /// Should the scanner enter this directory?
    #[must_use]
    pub fn allow_dir(&self, path: &Path, name: &str, is_hidden: bool) -> bool {
        if self.skip_hidden && is_hidden {
            return false;
        }
        if self.skip_dir_names.contains(name) {
            return false;
        }
        let path_str = path.to_string_lossy();
        for prefix in &self.skip_path_prefixes {
            if starts_with_ci(&path_str, prefix) {
                return false;
            }
        }
        true
    }

    /// Should the scanner emit this file?
    #[must_use]
    pub const fn allow_file(&self, is_hidden: bool, is_symlink: bool) -> bool {
        if self.skip_hidden && is_hidden {
            return false;
        }
        if self.skip_symlinks && is_symlink {
            return false;
        }
        true
    }
}

/// Case-insensitive `starts_with` for ASCII prefixes such as `C:\Windows`.
fn starts_with_ci(haystack: &str, needle: &str) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }
    haystack
        .chars()
        .zip(needle.chars())
        .all(|(a, b)| a.eq_ignore_ascii_case(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_skips_common_build_dirs() {
        let r = SkipRules::default();
        assert!(!r.allow_dir(&PathBuf::from("C:\\p\\node_modules"), "node_modules", false));
        assert!(!r.allow_dir(&PathBuf::from("C:\\p\\.git"), ".git", false));
    }

    #[test]
    fn default_skips_windows_system_paths_case_insensitively() {
        let r = SkipRules::default();
        assert!(!r.allow_dir(&PathBuf::from("c:\\windows\\system32"), "system32", false));
    }

    #[test]
    fn permissive_allows_everything() {
        let r = SkipRules::permissive();
        assert!(r.allow_dir(&PathBuf::from("C:\\p\\node_modules"), "node_modules", true));
        assert!(r.allow_file(true, true));
    }

    #[test]
    fn hidden_and_symlinks_are_skipped_by_default() {
        let r = SkipRules::default();
        assert!(!r.allow_file(true, false));
        assert!(!r.allow_file(false, true));
        assert!(r.allow_file(false, false));
    }
}
