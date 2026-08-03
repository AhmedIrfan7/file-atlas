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

#### M4. Duplicates

- `atlas-platform`: real `send_to_trash`/`restore_from_trash` via the `trash` crate (Windows Recycle Bin today; macOS/Linux ready for M8), with a manually-run integration test that roundtrips a real file through the real Recycle Bin
- `atlas-core`: `hasher` (BLAKE3, size-gated candidate selection so only files sharing a size with another file are read from disk), `duplicates` (grouping by hash with a keep-newest suggestion, ordered by wasted space), `safety` (self-healing protected-path guardrails seeded on every startup), `actions` (trash/restore backed by `actions_log`, guardrails enforced before any platform call)
- `atlas-desktop`: `hash_duplicates`/`cancel_hash` (with `hash-progress`/`hash-finished` events), `get_duplicate_groups`, `trash_selected_paths`, `restore_trash_action`, `list_recent_actions` commands; protected paths seeded in `AppState::new`
- UI: Duplicates tab with per-group keep-selection (radio, overridable), a two-step delete preview/confirm bar, and a "Recently deleted" panel with per-item restore
- ADR 0006 (safety pipeline implementation: why the `trash` crate over hand-rolled `IFileOperation`, why protected-path seeding is self-healing rather than respecting arbitrary row deletion, why undo is scoped to trash/restore only for now)
- 21 new unit/integration tests across the new modules (hasher, duplicates, safety, actions, trash_common), including a real (manually-run, `#[ignore]`d) Windows Recycle Bin roundtrip
- Verified end-to-end using a dedicated, throwaway scratch folder and a temporarily-swapped isolated database (the real 315 GB / 26,790-file index was backed up untouched beforehand): created two genuine duplicate pairs, ran a real hash pass, confirmed correct grouping and wasted-space totals, overrode the suggested keeper, previewed and confirmed a real delete (verified the files landed in the actual Windows Recycle Bin via `Shell.Application`), restored one from the "Recently deleted" panel (verified it reappeared on disk with byte-identical content), then restored the real database and confirmed the original 315 GB / 26,790 files / 3,912 folders were untouched
- Fixed a bug found during that verification: the Duplicates view did not refresh its group list when a hash pass finished

#### M5. Smart cleanup

- `atlas-recommender`: four rules against the existing schema, no new scanning or platform behavior needed — `empty_folders` (directories with zero live children, always safe, high confidence), `forgotten_installers` (category `Installer`, untouched 90+ days, medium confidence), `old_archives` (category `Archive`, untouched 180+ days, low confidence since some are intentional backups), `screenshot_pileups` (folders with 15+ screenshot-named images, one recommendation per folder, low confidence); `get_recommendations` merges all four with fixed default thresholds
- `atlas-desktop`: `get_cleanup_recommendations` command — the only new command M5 needed, since every recommendation's item paths feed straight into M4's existing `trash_selected_paths`/`restore_trash_action` pipeline unchanged
- UI: Cleanup tab with per-item checkboxes (confidence drives default selection: `High` pre-checked, `Medium`/`Low` left for manual review), per-group select-all/deselect-all, and the same delete preview/confirm bar and undo panel M4 built, reused as-is
- ADR 0007 (recommender rule scope): documents why two of the six originally-sketched rules — stale `node_modules` (excluded from the index entirely by M1's `SkipRules`, would need a dedicated size-aggregating sub-scan) and broken shortcuts (needs `.lnk` resolution behind `PlatformFs`) — are deferred rather than half-built into M5
- 7 new unit tests across `rules` and `engine`, all pure SQL reads against an in-memory database with no filesystem or platform I/O
- Verified two ways: (1) safely viewed real recommendations against the actual 315 GB / 26,790-file index (read-only, no execution) — genuinely found 90 real empty folders across the maintainer's real projects; (2) full isolated scratch-folder test with known, backdated test data (an old installer, an old archive, a recent installer that correctly did _not_ qualify, an empty folder, and a 16-file screenshot pileup) exercising every rule, select-all, delete-confirm, real Recycle Bin receipt, and restore — including trashing and restoring an empty _folder_, an edge case M4's file-only tests never covered

#### M6. Storage map

- `atlas-core`: `storage_map` module computing folder sizes on demand (no maintained rollup) via an escaped `LIKE` prefix scan against the existing `path` index, scoped to one drill-down level at a time (immediate children plus a synthetic "(files in this folder)" loose-file node), with `category` and `since_days` filters
- `atlas-desktop`: `get_storage_map_view` command, the only new command this milestone needed
- UI: Storage tab with a hand-rolled squarified treemap (Bruls/Huizing/van Wijk 1999, no charting library dependency), explicit breadcrumb-stack drill-down/back navigation, a category filter, and a four-preset time filter (All time / 7 days / 30 days / 1 year)
- Vitest wired up for the desktop app for the first time (previously deferred since M0/M2)
- ADR 0008 (on-demand `LIKE`-prefix aggregation vs. a maintained rollup table vs. recursive CTE; treemap vs. sunburst; fixed time presets vs. a continuous slider)
- 12 new unit tests: 5 in `atlas-core::storage_map` (including one proving folder names containing literal `%`/`_` are matched literally, not as wildcards) and 7 in the new `treemap.ts` test suite (area conservation, bounds containment, no negative/NaN dimensions, empty input)
- Two real bugs caught by these tests before they ever reached the app: a Rust SQL parameter-index mismatch between two functions sharing placeholder numbers, and a TypeScript treemap layout bug where row width/height and the remaining-bounds shrink direction were exactly swapped (4 of 7 tests failed until fixed)
- Verified end-to-end against the real 315 GB / 26,790-file index (safe to do directly since the command is purely read-only): treemap rendered real scan-root rectangles sized proportionally to real folder sizes, drill-down and breadcrumb-back navigation both worked, and the category and time-window filters correctly reduced the displayed totals
