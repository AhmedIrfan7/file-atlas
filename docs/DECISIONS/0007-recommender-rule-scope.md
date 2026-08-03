# ADR 0007: Recommender rule scope for M5

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

The original roadmap sketch for M5 listed six rule ideas: forgotten installers, screenshot pileups, stale `node_modules` in unopened projects, empty folders, broken shortcuts, and old cached ZIPs/ISOs. Implementing all six meant confronting two rules that do not fit cleanly on top of what M1 through M4 already built.

### Stale `node_modules` cannot be answered from the current index at all

`atlas_core::skip_rules::SkipRules::default()` (M1) skips `node_modules`, `.venv`, `target`, `__pycache__`, and similar build-cache directories by name during scanning. This was the right call for M1: these directories can contain hundreds of thousands of tiny files and dominate scan time for zero user-facing value. The consequence for M5 is that the index has no rows for `node_modules` folders at all, not even a directory entry. A rule like "find `node_modules` folders older than 30 days in projects untouched for 6 months" cannot be built from `SELECT`s against `files`; there is nothing to select.

Answering this rule for real requires a dedicated sub-scan that deliberately walks into build-cache directory names (inverting the default skip behavior for this one purpose), and then aggregating total size per folder, since `files` rows only carry individual file sizes, not folder totals. That is a real feature, not a query, and it deserves its own milestone-sized pass rather than being squeezed into M5 as a compromise.

### Broken shortcuts need platform-specific resolution

A `.lnk` file is an opaque binary format; telling whether it is "broken" means resolving what path it points to and checking whether that path still exists. Windows exposes this through `IShellLink` (COM). This is exactly the kind of platform-specific concern `atlas-platform`'s `PlatformFs` trait exists to isolate, similar to how `send_to_trash` was added in M4. Doing it well means a new trait method, a Windows implementation, and (eventually) macOS alias / Linux symlink equivalents. That is real, valuable work; it just is not a SQL query against the existing schema, and folding it into M5 alongside the four schema-only rules would have meant either rushing the platform work or letting it drag the whole milestone out.

## Decision

M5 ships four rules that are genuinely answerable from the current schema, each a straightforward, well-tested SQL query with no new scanning or platform behavior required:

- **Empty folders** — directories with zero live children. Always safe; highest confidence.
- **Forgotten installers** — files classified `Installer`, untouched for 90+ days.
- **Old archives** — files classified `Archive`, untouched for 180+ days. Lower confidence: some archives are intentional long-term backups.
- **Screenshot pileups** — folders with 15+ screenshot-named image files. One recommendation per qualifying folder; lowest confidence, purely a "worth reviewing" nudge rather than a deletion suggestion.

Stale `node_modules` and broken shortcuts are deferred, not silently dropped. Both remain in `docs/ROADMAP.md`'s outer roadmap; when either is picked up, it gets sized and planned like any other milestone rather than being retrofitted into M5's rule set at the last minute.

## Consequences

Positive:

- Every M5 rule is a pure SQL read, unit-testable in milliseconds against an in-memory database, with no new scanning behavior or platform code to get wrong.
- Execution reuses `atlas_core::actions::trash_paths` from M4 unchanged. `atlas-recommender` never touches the filesystem or the trash; it only ever produces `Recommendation`s whose item paths are exactly what the existing trash pipeline already accepts. No new execute path, no new safety surface to review.
- The confidence field does double duty: it is both the trust signal shown to the user and the UI's default-selection heuristic (`High` pre-checked, `Medium`/`Low` left for manual review), avoiding a second, redundant field.

Negative / accepted limitations:

- Two of the six original rule ideas are not in M5. Anyone reading the roadmap literally would expect six; this ADR is the record of why four shipped and two did not, and what each of the other two actually requires.

## Related work

- `crates/atlas-recommender/src/{types,rules,engine}.rs`
- `apps/desktop/src-tauri/src/recommendation_commands.rs`
- ADR 0004 (skip rules vs protected paths) — the origin of the node_modules skip behavior this ADR builds on
- ADR 0006 (safety pipeline implementation) — the trash pipeline this milestone's recommendations feed into unchanged
