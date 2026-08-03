# Changelog

All notable changes to File Atlas will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it reaches v1.0.0.

## [Unreleased]

### Added

#### M0. Foundation

- MIT license, README, code of conduct, contributing guide, security policy
- Issue templates (bug report, feature request) and pull request template with safety checklist
- Architecture overview and roadmap (M0 through M10)
- Architecture Decision Record framework with ADR 0001 and ADR 0002 (Tauri + Rust core)
- Cargo workspace with 5 crates: `atlas-core`, `atlas-db`, `atlas-platform`, `atlas-search`, `atlas-recommender`
- pnpm workspace root
- Tauri v2 desktop shell with React + TypeScript, branded as "File Atlas"
- Tailwind CSS v4 with base design tokens
- Prettier, ESLint (flat config, typescript-eslint, react, hooks, refresh), rustfmt, clippy configs
- Cargo nextest configuration with default and CI profiles
- GitHub Actions workflows: `ci.yml` (rust + web), `build.yml` (Windows Tauri bundle), `codeql.yml`
- Dependabot for cargo, npm, github-actions
- Lefthook hooks: pre-commit (prettier, eslint, rustfmt), pre-push (typecheck, lint, format-check, clippy), commit-msg (commitlint)
- Commitlint enforcing Conventional Commits with File Atlas type list
- GitHub labels (type, area, priority, meta) and milestones (M0 through M10)

#### M1. Scanner and Index engine

- `atlas-db`: rusqlite connection with WAL mode, embedded migrations framework, migrations 0001 (schema) and 0002 (FTS5), row models (`FileRow`, `VolumeRow`, `ActionRow`), typed queries with idempotent upserts and rescan sweep helper
- `atlas-core`: `FileRecord` domain type, `SkipRules` with defaults for common build caches and Windows system paths, `scanner` module walking with cancellation and progress events, `indexer` batching scan events into SQLite with per-scan `scans` row lifecycle and `mark_removed_since` sweep
- `atlas-platform`: `PlatformFs` trait with `Volume` type, Windows implementation for `list_volumes`, `is_hidden`, `is_system` via the `windows` crate, stub for non-Windows platforms
- `atlas-cli`: `atlas` binary with `scan`, `stats`, `volumes`, and `search` commands; verified end-to-end scanning of the repo itself
- ADR 0003 (SQLite index with single-writer model), ADR 0004 (skip rules vs protected paths)
- 18 unit tests: connection open/migrate roundtrips, upsert idempotency, rescan removal sweep, scanner coverage, node_modules skip, cancellation, progress emission, Windows volume enumeration, indexer full-scan and rescan

#### M2. Home view

- `atlas-core`: extension-based `classifier` wired into the indexer so every row gets a `Category`; `analytics` module (`home_summary`, `top_largest`, `top_oldest`, `stale_bucket`); `default_roots` using the `dirs` crate for onboarding suggestions; `run_with_progress` callback hook on the indexer for live progress without polling
- `atlas-desktop`: `AppState` holding one mutex-guarded SQLite connection (ADR 0003's single-writer model in practice); Tauri commands `get_default_roots`, `start_scan`, `cancel_scan`, `is_scanning`, `get_home_summary`, `get_top_largest`, `get_top_oldest`, `get_stale_bucket`; `scan-progress` and `scan-finished` events; `tauri-plugin-dialog` for custom folder picking
- UI: onboarding wizard (suggested + custom roots), scanning view with live progress and cancel, home view (category breakdown with bars, largest/oldest file lists, "not touched in a year" bucket), Zustand store for screen/progress state, typed `invoke()` wrappers
- Verified end-to-end against the real Tauri app on the maintainer's own machine: 315 GB across 26,790 files and 3,912 folders scanned from Desktop/Downloads/Documents/Pictures/Videos/Music, correctly categorized, with working stale-file and top-N views

#### M3. Search

- `atlas-search`: filter DSL parser (`type:`, `in:`, `size>`/`size<`/`size>=`/`size<=` with b/kb/mb/gb/tb units, `age>`/`age<`/`age>=`/`age<=` with d/w/m/y units, quoted phrases, free text), pure SQL planner (FTS5 prefix-match queries for free text, structured `WHERE` clauses for filters, bm25 ranking when text is present, `modified_at DESC` otherwise), a runner executing planned queries against a connection, and saved-search CRUD (unique by name) backed by migration 0003
- `atlas-desktop`: `search_files`, `save_search`, `list_saved_searches`, `delete_saved_search` commands
- UI: persistent Home/Search nav bar, search view with debounced search-as-you-type, results list, save/run/delete of saved searches
- ADR 0005 (search filter DSL and FTS5 prefix matching, and why a trigram tokenizer was not adopted)
- 33 unit/integration tests across parser, planner, runner, and saved searches
- Verified end-to-end against the real indexed data (26,790 files): free-text search ("resume") returned real project files; combined filter (`type:pdf size>1mb`) returned only matching PDFs; save/re-run/delete of a saved search round-tripped correctly; nav bar toggled Home/Search both directions
