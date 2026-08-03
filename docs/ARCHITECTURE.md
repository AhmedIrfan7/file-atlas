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

| Command                                            | Purpose                                                                                                        |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `get_default_roots`                                | Suggested onboarding roots (Desktop, Downloads, Documents, Pictures, Videos, Music) that exist on this machine |
| `start_scan(roots)`                                | Kick off a background scan of every root; returns immediately, progress arrives via events                     |
| `cancel_scan`                                      | Request cancellation of the in-progress scan                                                                   |
| `is_scanning`                                      | Whether a scan is currently running                                                                            |
| `get_home_summary`                                 | Aggregate totals plus the category breakdown                                                                   |
| `get_top_largest(limit)` / `get_top_oldest(limit)` | Top-N file lists for the home view                                                                             |
| `get_stale_bucket(min_age_days, sample_limit)`     | Files not modified in at least `min_age_days`, with a sample                                                   |

Events emitted to the frontend: `scan-progress` (`{ root, files_seen, bytes_seen }`) during a scan, `scan-finished` (`{ roots_scanned, total_entries_persisted, total_removed_marked, total_errors, cancelled }`) once.

`AppState` (`apps/desktop/src-tauri/src/state.rs`) holds the one SQLite connection behind a `parking_lot::Mutex`, plus `scan_cancel` and `scan_running` atomics. This is the concrete instance of the single-writer model from ADR 0003: the same connection serves both the indexer's writes during a scan and the analytics reads between scans, so there is never a "database is locked" race. The tradeoff, accepted for now, is that a long scan holds the lock for its duration; read commands issued mid-scan simply wait. A connection pool for concurrent reads during a scan is a candidate improvement once scans against multi-million-file volumes make that wait noticeable.

## Safety pipeline

Every destructive operation (`trash`, `permanent_delete`, `move`, `rename`, `bulk_move`) goes through:

```
request -> guardrails -> preview -> confirm -> execute -> action_log -> undo affordance
```

- **guardrails** rejects operations on protected paths, oversize, or high-risk targets. Returns a decision object with human-readable reasons.
- **preview** materializes what will happen (paths, sizes, warnings) for the UI.
- **confirm** requires an explicit UI click. Above thresholds, a typed confirmation.
- **execute** performs the operation. Deletes go to the OS Recycle Bin/Trash unless the user explicitly took the "permanent" path.
- **action_log** writes a row per mutation, before and after, with enough metadata to undo.
- **undo affordance** exposes a "restore" button for a rolling window.

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
