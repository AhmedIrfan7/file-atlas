# ADR 0006: Safety pipeline implementation (trash, guardrails, undo)

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

M4 is the first milestone that actually deletes anything. Up to this point `docs/ARCHITECTURE.md` described the safety pipeline (`request -> guardrails -> preview -> confirm -> execute -> action_log -> undo affordance`) as an intent; nothing implemented it. Three concrete questions had to be answered before a single file could be moved to the Recycle Bin:

1. How do we actually move a file to the OS trash and, later, bring it back?
2. How do we stop a bug (in our code or a future feature) from trashing something in `C:\Windows`?
3. How much "undo" do we build, given File Atlas will eventually have many kinds of actions (move, rename, trash, and later bulk operations)?

## Decisions

### Use the `trash` crate, not hand-rolled Win32 COM calls

We considered implementing `IFileOperation` (the modern, non-deprecated Windows API for Explorer-style file operations) directly via the `windows` crate, which we already depend on. We rejected it for M4: `IFileOperation` requires COM initialization, several interface implementations, and careful HRESULT handling, all for a single call. The `trash` crate wraps exactly this (and the macOS/Linux equivalents) and is used by real applications. For a safety-critical path, less custom code means less custom code that can be wrong.

The tradeoff: the crate's simple API (`trash::delete`) does not return a handle for what it just moved, because the underlying platform doesn't consistently expose one at delete time. To make `restore_from_trash` possible, we capture the original parent folder, file name, and the trash listing's `time_deleted` immediately after a successful delete, and use those three fields to re-find the item later via `trash::os_limited::list()`. This is implemented in `atlas-platform::trash_common` and is the same code path Windows uses today and macOS/Linux will use once M8 wires them up.

We verified this actually works, not just that it compiles: `crates/atlas-platform/src/trash_common.rs` has a `#[ignore]`d integration test (`send_then_restore_roundtrips`) that creates a real scratch file, sends it to the real Windows Recycle Bin, and restores it, run manually with `cargo test -- --ignored` rather than in CI, since CI should not manipulate a Recycle Bin. It passed on the first real run.

### Protected paths are self-healing, not "insert once"

`atlas_core::safety::seed_defaults` uses `INSERT OR IGNORE`, called on every app startup. We initially wrote a test asserting that if a user deleted the `C:\ProgramData` protection row, re-seeding would respect that removal. That test encoded the wrong design: a protected-path row disappearing (whether by an intentional user action reachable from nowhere in the UI today, a bug, or manual database editing) must never silently downgrade to "that system directory is now unprotected." We rewrote the test and the behavior to be self-healing: missing default rows always come back. Rows that still exist, including ones with user-edited `reason` text, are left alone.

This is a deliberate asymmetry: we optimize protected-path enforcement for "fails safe" over "respects every possible database edit." There is currently no UI path to remove a default protection, so this asymmetry costs nothing in practice today.

### Undo is scoped to trash/restore only, not a general action-reversal framework

`actions_log` (schema from M1) can represent any operation. We considered building a generic "undo the last N actions of any type" system now, since the table already supports it. We rejected that scope for M4: the only action type that exists is trash, so a general framework would be speculative code with no second use case to validate its shape against. `atlas_core::actions::restore_action` reverses exactly one trash action by id, and the UI surfaces this as a "recently deleted" list with per-item restore buttons. When move/rename actions exist (a later milestone), we will look at what undo actually needs to look like for those before generalizing.

### Hashing is decoupled from scanning

`atlas_core::hasher::hash_pending_duplicates` is a separate, explicitly-triggered pass, not part of the M1 scan flow. Hashing is size-gated (only files whose size collides with another live file are read and hashed) but still means reading full file contents from disk, which can be slow for large files. Running this automatically on every scan would make the fast, metadata-only scan experience from M1/M2 unpredictably slower as a user's data grows. The user opens the Duplicates tab and explicitly asks for it.

## Consequences

Positive:

- The core safety guarantee (never touch `C:\Windows`, `C:\Program Files`, etc.) is enforced in one place (`atlas_core::safety::check_paths`) that both the CLI and desktop app will call before any destructive operation, present or future.
- Restore is real and verified, not aspirational.
- Undo scope stayed small and shippable instead of speculative.

Negative / accepted limitations:

- Protected-path management has no UI yet (add, remove, edit reason). The self-healing seed function is the only way defaults change today.
- `restore_from_trash`'s re-find-by-fields approach could, in a rare race, restore the wrong item if two files with the identical name were deleted from the identical folder within the same second. This is a real if narrow edge case; revisit if the `trash` crate ever exposes a direct handle from `delete()`.

## Related work

- `crates/atlas-platform/src/trash_common.rs`
- `crates/atlas-core/src/{hasher,duplicates,safety,actions}.rs`
- `apps/desktop/src-tauri/src/duplicate_commands.rs`
- ADR 0003 (SQLite single-writer model) — trash/restore share the same mutex-guarded connection as everything else
- ADR 0004 (skip rules vs protected paths) — this ADR implements the enforcement side of that design
