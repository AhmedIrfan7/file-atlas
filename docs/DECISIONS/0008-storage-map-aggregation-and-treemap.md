# ADR 0008: Storage map aggregation and treemap choice

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

M6 needs to answer "where did my disk space go?" visually. Three design questions had to be settled: how folder sizes get computed (nothing stores them today), which visualization shape to use, and how literally to take "time slider" from the roadmap sketch.

### Folder sizes are not stored anywhere

`files` rows carry a leaf file's own `size_bytes`; directory rows are always `0` (`FileRecord::from_metadata`, M1). A treemap needs "how much does this folder weigh, including everything inside it," which nothing in the schema currently answers directly.

Two approaches were on the table:

1. **On-demand aggregation at read time.** For a folder `F`, sum `size_bytes` over every live file whose `path` starts with `F` (plus `F` itself), using an escaped `LIKE 'F\%' ESCAPE '!'` prefix scan against the existing unique index on `path`. Computed fresh per treemap request, scoped to just the handful of children being displayed at the current drill-down level.
2. **A maintained rollup table**, updated incrementally as files are scanned, hashed, trashed, or restored, so folder sizes are always precomputed and reads are O(1) lookups.

We chose (1). A treemap only ever needs sizes for the children of ONE folder at a time (a handful to a few dozen rows), not the whole tree at once, so the "expensive" part of aggregation is bounded by what is on screen, not by total index size. A maintained rollup would need every mutation path in the codebase (indexer upserts, trash, restore) to correctly update every ancestor's rollup, which is real bookkeeping complexity and a new class of consistency bug (a missed update silently makes a folder's displayed size wrong) for a win that only matters at a scale we have not hit yet.

If this approach is measurably too slow once someone has millions of files under a single deeply-nested folder, the documented next step is exactly that rollup table, built as its own milestone-sized change with its own consistency tests, not retrofitted quietly into M6.

### LIKE-prefix escaping: worth getting right, not worth skipping

A naive `LIKE 'F%'` pattern is wrong twice over: it can match unrelated sibling folders whose name happens to start with `F`'s name (`Desktop` vs `Desktop2`, fixed by always appending the path separator before the wildcard: `'F\%'`), and it treats any literal `%` or `_` in a real folder name as a wildcard, silently inflating or shrinking a folder's reported size for anyone whose folder happens to contain those characters (`"100%_done"`, not a hypothetical - version-control merge folders and downloaded archives use names like this routinely). We use SQLite's `ESCAPE` clause with `!` as the escape character (backslash cannot be the escape character here since it is the Windows path separator) and escape `%`, `_`, and `!` itself before building the pattern. This is covered by a unit test with a folder deliberately named `100%_done` next to a similarly-prefixed sibling file, which fails clearly if the escaping regresses.

### Treemap, not sunburst

Both were on the roadmap's shortlist. A treemap (nested rectangles sized by area) was chosen because it is what every disk-usage tool doing this well already converges on (WinDirStat, TreeSize, SpaceSonar): area is a more intuitive proxy for "how big" than angle or radius, comparing two rectangles by eye is easier than comparing two arc lengths, and "click a rectangle to descend" is a more natural interaction than picking a ring segment. It is also simpler to implement correctly from scratch: a squarified treemap layout (Bruls, Huizing, van Wijk 1999) is one well-understood recursive algorithm; a sunburst needs real SVG arc math for comparable visual quality. We wrote our own squarify implementation (`apps/desktop/src/lib/treemap.ts`) rather than pulling in a charting library, since it is roughly eighty lines of pure layout math with no rendering opinions to fight. It ships with its own unit tests (area conservation, bounds containment, no negative/NaN dimensions) precisely because layout math is exactly the kind of code that looks fine visually while being subtly wrong; the tests caught a real width/height axis inversion bug during development before it ever reached the browser.

### Time filter is four fixed presets, not a continuous slider

The roadmap sketch said "time slider." We shipped a segmented control: All time / 7 days / 30 days / 1 year, filtering every size computation to files with `modified_at >= cutoff`. A true continuous slider needs a defensible mapping from drag position to date range, live-updating query results as the user drags (meaning debounced re-fetching or client-side re-aggregation), and UI chrome (tick marks, a readable current-value label) that a fixed set of well-chosen presets does not. The four presets answer the same real question ("what changed recently") with less code and no interaction ambiguity. If continuous scrubbing turns out to matter once people are using this daily, it is a UI-only change on top of the same `since_unix` filter parameter that already exists end to end.

## Decision

Ship on-demand LIKE-prefix aggregation (with correct escaping), a hand-rolled squarified treemap, and a four-preset time filter, all as one coherent `atlas_core::storage_map` module plus a single `get_storage_map_view` Tauri command reused across every drill-down level and filter combination.

## Consequences

Positive:

- No new scan-time bookkeeping, no new consistency surface between the indexer/trash/restore paths and a rollup table.
- One SQL module (`storage_map.rs`), one command, one frontend store; the whole feature is small and auditable.
- The treemap layout bug the tests caught is a concrete example of why "pure logic, worth a real test" was the right call here, not a formality.

Negative / accepted limitations:

- Every drill-down triggers fresh `LIKE`-scan queries; for a folder with an enormous number of live descendants this is slower than a rollup lookup would be. Not yet measured against a truly enormous (multi-million-file) single subtree; revisit with real numbers if it becomes a complaint.
- The time filter cannot express "between two arbitrary dates," only "in the last N days." Documented as the reason a continuous slider was not built, not silently dropped.

## Related work

- `crates/atlas-core/src/storage_map.rs`
- `apps/desktop/src-tauri/src/storage_commands.rs`
- `apps/desktop/src/lib/treemap.ts` and its test file
- ADR 0003 (SQLite single-writer model) — storage map reads share the same mutex-guarded connection as everything else
