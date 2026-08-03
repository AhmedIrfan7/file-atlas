//! atlas-recommender
//!
//! A rule engine that produces cleanup recommendations from the index. Every
//! recommendation carries a plain-English reason and a confidence score so
//! the user can trust or dismiss it with full context. Rules never mutate
//! anything; execution is always deferred to `atlas-core::actions` with an
//! explicit user confirmation, reusing the same `trash_paths` pipeline M4
//! built for duplicates.
//!
//! ## Module map
//!
//! - `types` `Recommendation`, `RecommendationItem`, `Confidence`
//! - `rules` individual rule definitions (empty folders, forgotten
//!   installers, old archives, screenshot pileups)
//! - `engine` runs every rule and merges results with fixed default
//!   thresholds
//!
//! See `docs/DECISIONS/0007-recommender-rule-scope.md` for which rules from
//! the original roadmap sketch are deferred, and why.

#![doc(html_root_url = "https://docs.rs/atlas-recommender/0.1.0")]

pub mod engine;
pub mod rules;
pub mod types;

pub use engine::{
    get_recommendations, ARCHIVE_MIN_AGE_DAYS, INSTALLER_MIN_AGE_DAYS, SCREENSHOT_PILEUP_MIN_COUNT,
};
pub use rules::{empty_folders, forgotten_installers, old_archives, screenshot_pileups};
pub use types::{Confidence, Recommendation, RecommendationItem};
