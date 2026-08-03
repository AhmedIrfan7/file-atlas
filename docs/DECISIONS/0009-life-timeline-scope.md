# ADR 0009: Life timeline scope and burst detection

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

M7's roadmap sketch listed four things: a timeline of file creation, auto-grouping into "project bursts, screenshot bursts, receipt clusters, semester periods," and "This week" / "This year" views. Three scoping questions had to be settled before writing any code.

### Bucketing in SQL, not in Rust

The timeline is a histogram of `created_at` grouped into day or month periods. Two ways to build it: pull every live file's `created_at` across the FFI boundary and bucket it in Rust, or let SQLite do the grouping with `strftime(created_at, 'unixepoch', 'start of day' | 'start of month')` and only return the aggregates. We chose SQL grouping, same reasoning as M6's storage map: the expensive part (touching every row) should happen once, inside SQLite, with only the resulting handful of buckets crossing into Rust. `CAST(... AS INTEGER)` is required on the `strftime` result because SQLite's `strftime` returns text, and `rusqlite` will not silently coerce a text column into an `i64` on `row.get`.

### Only two granularities: day and month

The roadmap phrase was "This week and This year views." A generic day/week/month/quarter granularity picker was on the table, but a real week boundary needs its own SQLite date-modifier gymnastics (there is no `'start of week'` modifier; a Monday-start week has to be computed from `'weekday N'` arithmetic, and getting the off-by-one wrong is exactly the kind of bug M6 already caught twice this project for a different reason). Since the UI only ever needs two named views, `Granularity` has exactly two variants, `Day` and `Month`, both backed by an exact SQLite modifier with no custom date math. If a real need for week or quarter buckets shows up, it is a focused follow-up, not a speculative surface built now.

### Fixed view presets, not a generic date-range picker

Same reasoning as M6's time-window filter (ADR 0008): "This week" (day granularity, last 7 days), "This year" (month granularity, last 365 days), and "All time" (month granularity, no lower bound) are three fixed presets in a segmented control, reusing the exact interaction pattern the storage map already established. A calendar range picker would need its own UI chrome and validation for a use case ("see my whole digital life at a glance") that three well-chosen presets already serve.

### Two burst detectors, not four

The roadmap named four cluster types. Two are built:

- **Screenshot bursts**: days where at least `SCREENSHOT_BURST_MIN_COUNT` (6) screenshot-named images were created, anywhere in the index. Reuses the exact filename heuristic from M5's `screenshot_pileups` rule (`screenshot%`, `screen shot%`, `screen_shot%`, case-insensitive), but clusters by creation day across all folders instead of by folder across all time. A pileup answers "this folder has accumulated a lot of screenshots"; a burst answers "you took a lot of screenshots in one sitting."
- **Project bursts**: folder-and-day pairs where at least `PROJECT_BURST_MIN_COUNT` (8) files of any category were created in the same parent folder on the same day. The signature of extracting an archive, cloning a repository, or a batch download landing all at once.

Two are deliberately not built yet:

- **Receipt clusters** would need to tell a receipt or invoice apart from any other document, and a screenshot-style filename heuristic is far weaker here: screenshots have a near-universal OS-assigned naming convention across Windows, macOS, and most screenshot tools, while receipts arrive as `IMG_4821.jpg`, `invoice.pdf`, `Order_confirmation.pdf`, scanned photos, forwarded emails saved as PDF, and everything in between, in whatever language the vendor used. A filename-pattern rule here would have a high false-negative rate and a real chance of false positives on ordinary documents. This needs actual content signal (extracted text, not just the filename), which is exactly what M9's local AI layer (embeddings, extracted text from PDF/DOCX) is for. Building a weak version now and replacing it later would mean shipping a feature that quietly under-delivers on its name.
- **Semester periods** would need to know what a semester is: when it starts, when it ends, and for which institution or country, none of which is inferable from filesystem metadata. The closest structural proxy (a long span of `Document`/`Code` creation activity) is indistinguishable from any other months-long project without external calendar knowledge. Building this without that knowledge would mean guessing at institution-specific boundaries, which is worse than not building it.

Both deferrals follow the same precedent as ADR 0007 (recommender rule scope): ship what a SQL query against the current schema can answer honestly, defer what needs either new data (extracted text) or external knowledge (an academic calendar) the project does not have yet.

### Burst thresholds are fixed constants, not user-configurable

`SCREENSHOT_BURST_MIN_COUNT`, `PROJECT_BURST_MIN_COUNT`, and `BURST_SAMPLE_LIMIT` are `pub const`s in `atlas_core::timeline`, exactly mirroring `atlas_recommender::engine`'s `INSTALLER_MIN_AGE_DAYS` and friends. The Tauri commands for bursts take no parameters beyond `AppState`, the same shape as `get_cleanup_recommendations`. A tuning UI is speculative surface until real usage shows a threshold is wrong; at that point it is a one-line constant change, not a design problem.

## Decision

Ship `atlas_core::timeline` with SQL-side day/month bucketing (migration 0004 adds the `created_at` index this relies on), two named view presets reusing M6's segmented-control pattern, and two burst detectors (screenshot, project) with fixed thresholds. Defer receipt clusters to M9 (needs real content extraction) and semester periods indefinitely (needs external calendar knowledge this project does not have a source for).

## Consequences

Positive:

- No custom week-boundary date math to get subtly wrong.
- Burst detection reuses proven patterns (M5's filename heuristic, M6's threshold-constant style) instead of inventing new ones.
- The deferred features are deferred for structural reasons (missing data, missing external knowledge), not because they were hard to code, so the ADR gives a real trigger for revisiting them (M9 shipping text extraction; never, for semester periods, without a source of institutional calendars).

Negative / accepted limitations:

- No arbitrary date-range view; only the three fixed presets.
- Both burst types can produce long lists on a machine with genuinely heavy activity: verified against the maintainer's real 315 GB index, `screenshot_bursts` returned dozens of qualifying days going back years (a real, heavy screenshot habit, not a bug), and `project_bursts` correctly caught a real multi-thousand-file game install as one cluster per asset subfolder. The `min_count` thresholds are the only lever for now; if the lists prove too long to be useful, the fix is higher constants or pagination, not a redesign.

## Related work

- `crates/atlas-core/src/timeline.rs`
- `crates/atlas-db/migrations/0004_timeline_index.sql`
- `apps/desktop/src-tauri/src/timeline_commands.rs`
- ADR 0007 (recommender rule scope) — the precedent for deferring rules that need more than the current schema can honestly answer
- ADR 0008 (storage map aggregation and treemap) — the precedent for SQL-side aggregation and fixed time-window presets
