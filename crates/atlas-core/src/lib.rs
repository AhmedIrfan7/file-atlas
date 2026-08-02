//! atlas-core
//!
//! The heart of File Atlas. This crate owns the domain model and orchestrates
//! filesystem traversal, indexing, hashing, classification, action execution,
//! safety guardrails, and undo. Every destructive operation performed by the
//! application ultimately routes through this crate.
//!
//! ## Module map (populated across milestones)
//!
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
