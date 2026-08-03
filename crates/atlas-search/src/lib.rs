//! atlas-search
//!
//! Query the index. Combines SQLite FTS5 for name and path text with a small
//! filter DSL for structured constraints such as `type:pdf`, `size>10mb`,
//! `age>1y`, and `in:downloads`. Queries are pure functions over the index.
//! No side effects, no filesystem access, no destructive operations.
//!
//! ## Module map (populated across milestones)
//!
//! - `parser` filter DSL to AST
//! - `planner` AST to SQL
//! - `runner` execute SQL and return typed results
//! - `saved` persistence for user-saved searches

#![doc(html_root_url = "https://docs.rs/atlas-search/0.1.0")]

pub mod parser;
pub mod planner;

pub use parser::{parse, Cmp, Filter, ParseError, SearchQuery};
pub use planner::{plan, Param, PlannedQuery};
