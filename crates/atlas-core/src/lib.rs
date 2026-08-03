//! atlas-core
//!
//! The heart of File Atlas. This crate owns the domain model and orchestrates
//! filesystem traversal, indexing, hashing, classification, action execution,
//! safety guardrails, and undo. Every destructive operation performed by the
//! application ultimately routes through this crate.
//!
//! ## Module map (populated across milestones)
//!
//! - `file_record` the domain type emitted by the scanner
//! - `scanner` filesystem traversal with skip rules
//! - `indexer` batched writes into the SQLite index
//! - `hasher` background BLAKE3 hashing pipeline
//! - `classifier` category tagging for files and folders
//! - `actions` move, rename, trash, restore
//! - `safety` guardrails and protected-path enforcement
//! - `undo` action log and reversal
//! - `analytics` storage totals, aging, breakdowns
//! - `recommender` explainable cleanup suggestions
//!
//! No module accesses the operating system directly. All platform-specific
//! behavior lives in `atlas-platform` behind the `PlatformFs` trait.

#![doc(html_root_url = "https://docs.rs/atlas-core/0.1.0")]

pub mod analytics;
pub mod classifier;
pub mod default_roots;
pub mod duplicates;
pub mod file_record;
pub mod hasher;
pub mod indexer;
pub mod scanner;
pub mod skip_rules;

pub use analytics::{
    home_summary, stale_bucket, top_largest, top_oldest, CategoryTotal, FileSummary, HomeSummary,
    StaleBucket,
};
pub use classifier::{classify, Category};
pub use default_roots::{default_roots, SuggestedRoot};
pub use duplicates::{find_duplicate_groups, DuplicateGroup, DuplicateMember};
pub use file_record::FileRecord;
pub use hasher::{hash_pending_duplicates, HashProgress, HashStats, MIN_HASH_SIZE_BYTES};
pub use indexer::{
    record_volume, root_prefix, run as index_run, run_with_progress as index_run_with_progress,
    IndexError, IndexProgress, IndexStats, ScanMeta,
};
pub use scanner::{scan, ScanConfig, ScanEvent, ScanReport};
pub use skip_rules::SkipRules;
