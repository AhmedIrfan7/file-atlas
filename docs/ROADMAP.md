# Roadmap

Each milestone is a small, coherent step. We do not begin the next milestone until the current one is committed, tested, documented, and shipped.

## M0. Foundation

Goal: a clean, professional repository skeleton that a stranger can navigate on day one.

- Repository, license, README, code of conduct, contributing guide, security policy
- Issue and PR templates
- Architecture and roadmap skeletons
- ADR structure
- Cargo workspace with five empty crates
- pnpm workspace
- Tauri app scaffold with a Hello World window
- Formatters, linters, type checkers, test runners
- CI: lint, test, build (Windows)
- CodeQL, Dependabot, pre-commit hooks, commitlint

## M1. Scanner and index engine

Goal: a headless engine that can enumerate a directory tree and populate a SQLite index correctly, fast, and safely.

- Volume and drive enumeration
- Directory walker with skip rules
- FileRecord model and channel plumbing
- Indexer with batched upserts
- Incremental rescan with `last_seen`
- Migrations framework
- CLI harness for engine-only testing
- Unit tests for edge cases: unicode, long paths, symlinks, hidden files, permission errors

## M2. Home view

Goal: the first user-facing screen. A visitor understands their disk within seconds.

- Onboarding wizard: pick roots (Desktop, Downloads, Documents, Pictures, Videos, Music by default)
- Scan progress with live counts
- Folder tree with size totals
- Category breakdown (Images, Videos, Documents, Archives, Installers, Code, Other)
- Top N largest and top N oldest lists
- "Not touched in over a year" bucket

## M3. Search

Goal: find anything in the index in under 100ms on a warm cache.

- FTS-backed name and path search
- Structured filters: `type:pdf`, `size>10mb`, `age>1y`, `in:downloads`
- Saved searches
- Empty and error states

## M4. Duplicates

Goal: safely identify duplicate files and let the user reclaim space in one click.

- Hashing pipeline (BLAKE3), size-gated and extension-gated
- Duplicate grouping
- Safe-choice UI: keep newest by default, show why, allow override
- Bulk trash with preview and undo
- Explanation for every duplicate group

## M5. Smart cleanup

Goal: transparent, explainable recommendations. No black boxes.

- Rule engine (shipped: empty folders, forgotten installers, old archives, screenshot pileups)
- Each recommendation shows its reason and its confidence
- Every recommendation is reviewable; execution reuses M4's trash/undo pipeline unchanged
- Deferred, not dropped: stale `node_modules`/build caches (needs a dedicated sub-scan that indexes what `SkipRules` currently excludes, plus folder-size aggregation) and broken shortcuts (needs `.lnk` resolution behind `PlatformFs`). See ADR 0007.

## M6. Storage map

Goal: see storage at a glance. Understand where the space actually went.

- Treemap or sunburst of storage by folder
- Drill-down, filter by category
- Time slider showing what changed over the last week, month, year

## M7. Life timeline

Goal: a chronological view of your digital life.

- Timeline of file creation
- Auto-grouping: project bursts, screenshot bursts, receipt clusters, semester periods
- "This week" and "This year" views

## M8. macOS and Linux support

Goal: parity with Windows on Mac and Linux.

- `atlas-platform` implementations for macOS and Linux
- Trash integration
- Shell integration
- CI build matrix expanded

## M9. Local AI layer

Goal: natural-language search and smart suggestions, without sending data off-device by default.

- Local embedding index (file names, extracted text from PDF/DOCX)
- Natural-language query to SQL translation
- Optional local LLM integration (Ollama)
- Optional cloud LLM path, opt-in, clearly labeled, per-request confirmation

## M10. v1.0

Goal: a shippable public release.

- Autoupdater
- Signed installers
- Landing page
- Screenshots, demo video
- Release notes
- v1.0.0 tag
