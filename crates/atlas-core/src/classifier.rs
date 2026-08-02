//! Category tagging for files.
//!
//! Categorization is extension-based for M2. It is deliberately simple: a
//! lookup table from lowercase extension to a `Category` enum. Directory
//! entries and files with unknown extensions fall back to sensible defaults.
//! Smarter classification (content sniffing, project detection, semantic
//! grouping) arrives in later milestones and layers on top of this, it does
//! not replace it.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Coarse categories shown in the home view's breakdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Image,
    Video,
    Audio,
    Document,
    Archive,
    Installer,
    Code,
    Folder,
    Other,
}

impl Category {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Image => "Image",
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Document => "Document",
            Self::Archive => "Archive",
            Self::Installer => "Installer",
            Self::Code => "Code",
            Self::Folder => "Folder",
            Self::Other => "Other",
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Classify a path. Directories are always `Category::Folder`. Files are
/// classified by their lowercase extension; unknown extensions fall back to
/// `Category::Other`.
#[must_use]
pub fn classify(path: &Path, is_dir: bool) -> Category {
    if is_dir {
        return Category::Folder;
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    match ext.as_deref() {
        Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "heic" | "svg" | "tiff" | "ico") => {
            Category::Image
        }
        Some("mp4" | "mov" | "mkv" | "avi" | "webm" | "wmv" | "flv" | "m4v") => Category::Video,
        Some("mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma") => Category::Audio,
        Some(
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" | "rtf" | "odt"
            | "csv",
        ) => Category::Document,
        Some("zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso") => Category::Archive,
        Some("exe" | "msi" | "msix" | "appx" | "dmg" | "pkg" | "deb" | "rpm") => {
            Category::Installer
        }
        Some(
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "java" | "c" | "cpp" | "h" | "hpp" | "go"
            | "rb" | "php" | "swift" | "kt" | "cs" | "json" | "yaml" | "yml" | "toml" | "html"
            | "css" | "scss" | "sql" | "sh" | "ps1",
        ) => Category::Code,
        _ => Category::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn directories_are_always_folder() {
        assert_eq!(
            classify(&PathBuf::from("C:\\anything.txt"), true),
            Category::Folder
        );
    }

    #[test]
    fn common_extensions_map_correctly() {
        assert_eq!(
            classify(&PathBuf::from("photo.JPG"), false),
            Category::Image
        );
        assert_eq!(
            classify(&PathBuf::from("movie.mkv"), false),
            Category::Video
        );
        assert_eq!(classify(&PathBuf::from("song.mp3"), false), Category::Audio);
        assert_eq!(
            classify(&PathBuf::from("resume.pdf"), false),
            Category::Document
        );
        assert_eq!(
            classify(&PathBuf::from("archive.zip"), false),
            Category::Archive
        );
        assert_eq!(
            classify(&PathBuf::from("setup.exe"), false),
            Category::Installer
        );
        assert_eq!(classify(&PathBuf::from("main.rs"), false), Category::Code);
    }

    #[test]
    fn unknown_extension_is_other() {
        assert_eq!(
            classify(&PathBuf::from("mystery.xyz123"), false),
            Category::Other
        );
    }

    #[test]
    fn no_extension_is_other() {
        assert_eq!(classify(&PathBuf::from("README"), false), Category::Other);
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(Category::Image.to_string(), "Image");
    }
}
