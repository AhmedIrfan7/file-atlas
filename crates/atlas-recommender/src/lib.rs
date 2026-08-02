//! atlas-recommender
//!
//! A rule engine that produces cleanup recommendations from the index. Every
//! recommendation carries a plain-English reason and a confidence score so
//! the user can trust or dismiss it with full context. Rules never mutate
//! anything; execution is always deferred to `atlas-core::actions` with an
//! explicit user confirmation.
//!
//! ## Module map (populated across milestones)
//!
//! - `rules` individual rule definitions (forgotten installers, screenshot
//!   pileups, stale node_modules, empty folders, and so on)
//! - `engine` evaluator that runs rules against the index and merges results
//! - `explain` renders a human-readable rationale for a recommendation

#![doc(html_root_url = "https://docs.rs/atlas-recommender/0.1.0")]
