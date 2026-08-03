//! Shared types returned by every rule.
//!
//! Deliberately does not depend on `atlas-core` or `atlas-db` model types
//! (the same choice `atlas-search` made for `SearchHit`): this crate only
//! ever reads through raw `rusqlite::Connection`, and a handful of shared
//! fields do not justify a cross-crate dependency.

use serde::{Deserialize, Serialize};

/// How strongly a recommendation should be trusted. Drives the UI's default
/// selection: `High` confidence items are pre-checked for deletion, `Medium`
/// and `Low` are surfaced for manual review instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// One file or folder surfaced by a recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendationItem {
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
}

/// One actionable suggestion: a reason, a confidence level, and the items it
/// applies to.
///
/// Rules never execute anything; a recommendation's `items` paths are
/// exactly what a caller would hand to `atlas_core::actions::trash_paths`
/// once the user reviews and confirms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recommendation {
    pub kind: String,
    pub title: String,
    pub explanation: String,
    pub confidence: Confidence,
    pub total_bytes: i64,
    pub items: Vec<RecommendationItem>,
}
