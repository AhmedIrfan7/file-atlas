# Architecture

> Status: skeleton. This document evolves alongside the code. Every architectural change should update this file in the same PR.

## Goals

File Atlas is a local-first desktop application that indexes, understands, and helps reshape a user's filesystem. The architecture is optimized for four things, in order:

1. **Safety.** No accidental data loss, ever. Reversible operations by default.
2. **Performance.** Millions of files, terabytes of storage, on modest hardware.
3. **Cross-platform.** Windows, macOS, Linux from a shared core.
4. **Simplicity.** Small, well-named modules. No cleverness unless it earns its keep.

## High-level shape

```
+-----------------------------------------------------------+
|                        UI (React + TS)                     |
|   Views, search, storage map, actions, onboarding, prefs   |
+------------------------ Tauri IPC -------------------------+
|                                                            |
|   Command handlers (thin adapters, no business logic)      |
|                                                            |
+-----------------------------------------------------------+
|                      Core (Rust workspace)                 |
|                                                            |
|   scanner   indexer   hasher   classifier   actions        |
|   search    analytics   recommender   safety   undo        |
|                                                            |
+------------------------ Platform trait --------------------+
|   Windows impl    macOS impl    Linux impl                 |
+-----------------------------------------------------------+
|                    SQLite index (local file)               |
+-----------------------------------------------------------+
```

## Workspace layout

```
crates/
  atlas-core/          traversal, indexing, hashing, classification, actions, safety, undo
  atlas-db/            SQLite schema, migrations, typed queries
  atlas-platform/      trait plus per-OS implementations
  atlas-search/        full-text and structured search
  atlas-recommender/   rule-based cleanup suggestions
apps/
  cli/                 atlas-cli: engine harness (scan, stats, volumes, search commands)
  desktop/             Tauri app: Rust shell in src-tauri/, React UI in src/
docs/
  ARCHITECTURE.md      this file
  ROADMAP.md           milestone plan
  DECISIONS/           Architecture Decision Records (ADRs)
  USER_GUIDE.md        end-user documentation (added once the app is stable enough to document)
```

## Data flow: a scan

1. UI calls the `start_scan` Tauri command with a list of root paths (from onboarding or "Add more folders").
2. The command handler spawns a background thread and returns immediately; the UI listens for events rather than awaiting a result.
3. For each root, the handler resolves the owning `atlas_platform::Volume`, records it, then calls `atlas_core::scan` on a second thread. `scan` walks with `walkdir`, honoring `SkipRules` (`node_modules`, `.git`, hidden files unless opted in, Windows system paths), and pushes a `ScanEvent` per entry onto a `crossbeam_channel`.
4. `atlas_core::index_run_with_progress` consumes the channel on the command-handler thread: each `Entry` is classified (`atlas_core::classifier::classify`) and batched into SQLite upserts via `atlas_db::queries`; each `Progress` tick invokes a callback that emits a `scan-progress` Tauri event.
5. When every root is done, the handler emits one `scan-finished` event with aggregate totals.
6. The UI's Home view then calls `get_home_summary`, `get_top_largest`, `get_top_oldest`, and `get_stale_bucket` to render the map. These are pure reads against `atlas_core::analytics`.

Hashing (for duplicate detection) and semantic classification beyond file extension are not part of this flow yet; they arrive in M4 and M9 respectively.

## Tauri command surface

Commands live in `apps/desktop/src-tauri/src/commands.rs`. Each is a thin adapter: deserialize arguments, call into `atlas-core`/`atlas-db`/`atlas-platform`, serialize the result. No business logic lives in this file.

| Command                                                                             | Purpose                                                                                                        |
| ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `get_default_roots`                                                                 | Suggested onboarding roots (Desktop, Downloads, Documents, Pictures, Videos, Music) that exist on this machine |
| `start_scan(roots)`                                                                 | Kick off a background scan of every root; returns immediately, progress arrives via events                     |
| `cancel_scan`                                                                       | Request cancellation of the in-progress scan                                                                   |
| `is_scanning`                                                                       | Whether a scan is currently running                                                                            |
| `get_home_summary`                                                                  | Aggregate totals plus the category breakdown                                                                   |
| `get_top_largest(limit)` / `get_top_oldest(limit)`                                  | Top-N file lists for the home view                                                                             |
| `get_stale_bucket(min_age_days, sample_limit)`                                      | Files not modified in at least `min_age_days`, with a sample                                                   |
| `open_in_file_manager(path)`                                                        | Reveal `path` in Explorer / Finder / the default Linux file manager                                            |
| `search_files(query_text, limit)`                                                   | Parse `query_text` through `atlas_search::parser` and run it (see below)                                       |
| `save_search(name, query_text)` / `list_saved_searches` / `delete_saved_search(id)` | Saved-search CRUD, `apps/desktop/src-tauri/src/search_commands.rs`                                             |
| `hash_duplicates` / `cancel_hash`                                                   | Kick off / cancel a size-gated BLAKE3 hashing pass (`apps/desktop/src-tauri/src/duplicate_commands.rs`)        |
| `get_duplicate_groups(limit)`                                                       | Duplicate groups with a suggested keeper, most wasted space first                                              |
| `trash_selected_paths(paths)`                                                       | Execute step of the safety pipeline below: guardrails, send to OS trash, log, mark removed                     |
| `restore_trash_action(action_id)` / `list_recent_actions(limit)`                    | Undo affordance: reverse one trash action, or list recent ones for a "recently deleted" panel                  |
| `get_cleanup_recommendations`                                                       | Every current rule-engine recommendation (see below); execution reuses `trash_selected_paths` above            |
| `get_storage_map_view(path, category, since_days)`                                  | Folder-size breakdown for one drill-down level (see below)                                                     |
| `get_life_timeline(granularity, since_days)`                                        | File-creation histogram bucketed by day or month (see below)                                                   |
| `get_screenshot_bursts` / `get_project_bursts`                                      | Auto-detected creation-activity clusters (see below)                                                           |

Events emitted to the frontend: `scan-progress` (`{ root, files_seen, bytes_seen }`) / `scan-finished` during a scan; `hash-progress` (`{ files_hashed, files_total }`) / `hash-finished` (`{ files_hashed, errors }`) during a hash pass.

### Recommendations

`atlas_recommender::get_recommendations` runs four independent rules against the index (`empty_folders`, `forgotten_installers`, `old_archives`, `screenshot_pileups`) and merges the results. Every rule is a pure SQL read: no filesystem access, no mutation, and a `Recommendation`'s `items` are exactly the paths a caller would hand to `atlas_core::actions::trash_paths`. This means M5 needed no new execute path or safety surface; it produces candidates, M4's pipeline still does the deleting. `Confidence` (`High`/`Medium`/`Low`) is both the trust signal shown in the UI and the default-selection heuristic (`High` pre-checked, everything else left for manual review). See ADR 0007 for which two rules from the original roadmap sketch are deferred and why.

### Search

`search_files` sends `query_text` through `atlas_search`'s three pure stages before touching the database: `parser::parse` turns it into a `SearchQuery` (free text plus a `Vec<Filter>`), `planner::plan` turns that into parameterized SQL against `files` (and `files_fts` when there is free text), and `runner::search` executes it. Parser and planner have no I/O and are unit tested without a connection; only `runner`'s tests open a real (in-memory) SQLite database. See ADR 0005 for the filter DSL grammar and why free text uses FTS5 prefix queries rather than a trigram tokenizer.

`AppState` (`apps/desktop/src-tauri/src/state.rs`) holds the one SQLite connection behind a `parking_lot::Mutex`, plus `scan_cancel` and `scan_running` atomics. This is the concrete instance of the single-writer model from ADR 0003: the same connection serves both the indexer's writes during a scan and the analytics reads between scans, so there is never a "database is locked" race. The tradeoff, accepted for now, is that a long scan holds the lock for its duration; read commands issued mid-scan simply wait. A connection pool for concurrent reads during a scan is a candidate improvement once scans against multi-million-file volumes make that wait noticeable.

### Storage map

`get_storage_map_view` calls `atlas_core::storage_map::get_storage_map`, which computes folder sizes on demand rather than from a maintained rollup: given a scope path (or `None` for the root list of completed scan roots), it sums `files.size_bytes` for every live row whose path is the scope itself or starts with `scope\` (an escaped `LIKE` prefix scan against the existing `path` index), once per immediate child plus one synthetic "(files in this folder)" node for loose files. `category` and `since_days` filters narrow the same sum. See ADR 0008 for why this beats a maintained rollup at the scale of one drill-down level, and for the treemap-vs-sunburst and fixed-presets-vs-slider choices on the frontend.

The frontend (`apps/desktop/src/components/StorageMapView.tsx`) holds an explicit breadcrumb stack (`storageMapStore.ts`) rather than parsing path strings for "back" navigation, and lays out each response's nodes with a hand-rolled squarified treemap (`apps/desktop/src/lib/treemap.ts`, unit tested, no charting library dependency).

### Life timeline

`get_life_timeline` calls `atlas_core::timeline::get_timeline`, which buckets every live file's `created_at` into day or month periods entirely in SQL (`strftime(created_at, 'unixepoch', 'start of day' | 'start of month')`, migration 0004's `idx_files_created` index makes the grouping indexed rather than a full scan). `get_screenshot_bursts` and `get_project_bursts` call the two burst detectors in the same module: days with an unusual number of screenshot-named image creations anywhere in the index, and folder-and-day pairs with an unusual number of file creations at once. Both use fixed threshold constants (`SCREENSHOT_BURST_MIN_COUNT`, `PROJECT_BURST_MIN_COUNT`), the same style as `atlas_recommender::engine`'s fixed rule thresholds. See ADR 0009 for why only two burst types ship now (receipt clusters and semester periods are deferred, not dropped) and why granularity is day/month only, not a generic picker.

The frontend (`apps/desktop/src/components/TimelineView.tsx`) offers three fixed view presets, This week / This year / All time (`timelineStore.ts`), reusing the storage map's segmented-control pattern from ADR 0008, and renders the histogram as a plain CSS bar chart (`TimelineChart.tsx`) with burst results shown as cards (`BurstCard.tsx`).

## Safety pipeline

Every destructive operation goes through:

```
request -> guardrails -> preview -> confirm -> execute -> action_log -> undo affordance
```

Implemented for trash/restore as of M4 (see ADR 0006 for why each piece looks the way it does):

- **guardrails**: `atlas_core::safety::check_paths` rejects any path under a `protected_paths` prefix, seeded self-healingly on every startup by `atlas_core::safety::seed_defaults` so a missing row never quietly means "unprotected." The prefix list is per-OS as of M8 (`C:\Windows`, `C:\Program Files`, etc. on Windows; `/System`, `/Applications`, etc. on macOS; `/usr`, `/opt`, etc. on Linux), plus a runtime-resolved per-user trash folder on macOS/Linux. See ADR 0010 for why this had to change: the prefix list was previously Windows-only and silently protected nothing at all on macOS or Linux.
- **preview**: built client-side in `DuplicatesView`/`DeletePreviewBar` from the file list and total bytes before any command is called.
- **confirm**: an explicit second click in `DeletePreviewBar` (the button only sends the trash command after the user clicks "Confirm delete" on the preview).
- **execute**: `atlas_core::actions::trash_paths` calls `PlatformFs::send_to_trash`, implemented via the `trash` crate (`atlas_platform::trash_common`), which wraps the real Windows Recycle Bin / macOS Trash / Linux freedesktop trash. Restore (the next bullet) works on Windows and Linux; on macOS it reports `Unsupported` since the crate exposes no way to find a trashed item again by identity there. See ADR 0010.
- **action_log**: every successful trash writes one `actions_log` row (`op = 'trash'`) with enough metadata (parent, name, deletion timestamp, JSON-encoded in `metadata`) to find the item again in the OS trash listing later, and sets `files.removed_at`.
- **undo affordance**: `atlas_core::actions::restore_action` reverses exactly one trash action by id; the UI's "Recently deleted" panel lists recent ones with a Restore button per item. Undo is intentionally scoped to trash/restore only, not a general action-reversal framework, until a second action type exists to validate that shape against.

Deferred beyond M4: oversize/high-risk-target guardrails beyond protected paths, and permanent (non-trash) delete.

## Platform trait

```rust
pub trait PlatformFs {
    fn send_to_trash(&self, path: &Path) -> Result<TrashHandle, PlatformError>;
    fn restore_from_trash(&self, handle: &TrashHandle) -> Result<PathBuf, PlatformError>;
    fn list_volumes(&self) -> Result<Vec<Volume>, PlatformError>;
    fn is_hidden(&self, path: &Path) -> Result<bool, PlatformError>;
    fn is_system(&self, path: &Path) -> Result<bool, PlatformError>;
    fn open_in_file_manager(&self, path: &Path) -> Result<(), PlatformError>;
}
```

Core code never uses `#[cfg(target_os)]`. Implementations live in `atlas-platform` and are selected at startup.

As of M8, all three target platforms have real implementations (`windows_impl`, `macos_impl`, `linux_impl`); `stub_impl` remains only as a compile fallback for any other OS. `list_volumes` is real device enumeration on each OS (`GetLogicalDrives`/`GetVolumeInformationW` on Windows, `/Volumes` plus `libc::statfs` on macOS, `/proc/mounts` plus `libc::statvfs` on Linux). `is_hidden` and `is_system` follow each OS's real convention rather than a shared one: Unix has no per-file "system" attribute the way NTFS does, so `is_system` there is prefix-based instead. `open_in_file_manager` (Explorer `/select,` / Finder `open -R` / `xdg-open`) went from zero implementations and zero callers on any platform to a real "Show in folder" action wired into search results. See ADR 0010 for the per-platform reasoning, and for why `restore_from_trash` is the one method that is not full parity yet (`Unsupported` on macOS; see the safety pipeline section above).

## Storage

SQLite is the source of truth for the index. Design notes:

- Path is the natural key, but we assign an integer `id` for join efficiency.
- FTS5 virtual table for name and path search.
- Batched writes with `WAL` journal mode and `synchronous = NORMAL`.
- Volume id lets us recognize the same file across drive-letter shuffles or mounts.
- `first_seen` and `last_seen` timestamps let us model file lifetime and detect resurrections.

## Concurrency

- One SQLite connection behind a mutex per running app instance (see "Tauri command surface" above), matching ADR 0003's single-writer model. A pool for concurrent reads is a future improvement, not yet needed.
- The scanner runs on its own OS thread per root; the indexer consumes its channel on the command-handler thread.
- `crossbeam_channel` between scanner and indexer; unbounded today, revisit if a scan of a very large volume shows memory pressure.
- Cancellation is a shared `AtomicBool` checked by the scanner between entries and by `start_scan` between roots.

## Errors

- `thiserror` for library boundaries (typed, exhaustive).
- `anyhow` for the app shell only (bubble to UI as a user-facing message).
- Never `panic!` on user input or filesystem responses. Panics indicate a bug and are logged with backtrace.

## Logging

- `tracing` throughout.
- Structured logs (JSON) to a rolling file under the OS-appropriate log directory.
- Human-readable logs in dev builds.
- No file content ever logged. Only paths, sizes, categories, error kinds.

## Extension points (later milestones)

- **AI layer** as a separate crate with a trait for LLM providers. Local embedding by default. Cloud provider is opt-in and clearly labeled.
- **Plugins** consume a stable command API. Sandboxed by default.
- **Automation rules** are declarative documents evaluated by an engine crate.

## Non-goals

- Not a general-purpose file manager. We do not aim to replace Explorer or Finder.
- Not a cloud sync tool. We may integrate with existing cloud folders (OneDrive, Dropbox, iCloud) but we do not host anything.
- Not a security scanner. We may surface obvious risks (executables in Downloads) but we are not an antivirus.
