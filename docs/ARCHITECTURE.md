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
  desktop/             Tauri app: Rust shell in src-tauri/, React UI in src/
docs/
  ARCHITECTURE.md      this file
  ROADMAP.md           milestone plan
  DECISIONS/           Architecture Decision Records (ADRs)
  USER_GUIDE.md        end-user documentation (added at M2)
```

## Data flow: a scan

1. UI calls `scan_root(path)`.
2. Tauri command handler validates the path and asks `atlas-core::scanner` to enumerate.
3. `scanner` walks with `ignore::Walk`, honoring skip rules (`node_modules`, `.git`, hidden files unless user opts in, protected system paths).
4. Each entry becomes a `FileRecord` and is pushed on a bounded channel.
5. `atlas-core::indexer` consumes records and batches them into SQLite upserts via `atlas-db`.
6. When indexing an entry finishes, `atlas-core::hasher` may queue it (based on size, extension, dedup relevance).
7. `atlas-core::classifier` tags records post-index or on-demand.
8. UI subscribes to progress via a Tauri event channel.

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

- One Tokio runtime.
- One writer task for SQLite (SQLite is single-writer). Reads go through a connection pool.
- Bounded channels between scanner, indexer, and hasher to backpressure disk pressure.
- A cooperative cancellation token per scan job.

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
