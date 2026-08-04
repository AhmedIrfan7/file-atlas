# File Atlas v1.0.0

The living map of everything on your computer, out of pre-alpha and into its first real release.

File Atlas is a local-first desktop app that indexes, understands, and helps you safely reshape everything stored on your machine. Ten milestones got it here: a scanner and index engine, a home view, search, duplicate detection, explainable smart cleanup, a storage map, a life timeline, macOS/Linux support, a local AI layer, and now the release pipeline that ships all of it.

## Highlights

**Understand your disk in seconds.** The Home view breaks your files down by category, surfaces your largest and oldest files, and flags what you have not touched in over a year, all from a background scan that never blocks the UI.

**Find anything.** Full-text and structured search (`type:pdf`, `size>10mb`, `age>1y`, saved searches) runs in under 100ms against a warm index, backed by SQLite FTS5.

**Clean up safely.** Duplicate detection uses BLAKE3 hashing with a safe keep-newest default you can override. Smart Cleanup adds rule-based recommendations, empty folders, forgotten installers, old archives, screenshot pileups, each with a stated reason and confidence, never a silent automatic delete. Every destructive action goes through the same pipeline: guardrails, preview, explicit confirm, execute, and a "Recently deleted" panel with real undo through the OS trash.

**See where your space went.** A hand-rolled treemap of storage by folder and category, with drill-down and a time filter.

**See your digital life.** A chronological timeline of file creation, auto-grouped into project bursts and screenshot pileups.

**Cross-platform.** Real, tested `PlatformFs` implementations for Windows, macOS, and Linux, not a single Windows-only codepath with stubs elsewhere.

**Local AI, opt-in cloud.** Natural-language search and query translation run against a local embedding index by default, with an optional local LLM via Ollama. A cloud LLM path exists too, but it is opt-in, clearly labeled, and confirmed per request, never a default.

**Updates itself, safely.** File Atlas checks for new releases on launch and shows what's available; installing one always takes an explicit click.

## Known limitations

- Installers are not yet signed with a paid OS-level publisher certificate. Windows and macOS may show an unknown-publisher warning on first install. This does not affect the updater's own cryptographic signature, which is independently verified on every update.
- Restoring a trashed item through File Atlas's own UI is not available on macOS (the underlying `trash` crate has no macOS restore-by-identity API). The file is not at risk either way: macOS Finder's own "Put Back" still works.
- Natural-language query translation quality depends on the local model you run. Every result is still validated against the real search grammar before it runs, so a bad translation degrades to "no matches," never a wrong or unsafe one.

## Getting started

Download the installer for your platform from the [latest release](https://github.com/AhmedIrfan7/file-atlas/releases/latest), or read the [README](README.md) for a full feature rundown and the [architecture doc](docs/ARCHITECTURE.md) for how it's built.

See [CHANGELOG.md](CHANGELOG.md) for the complete, milestone-by-milestone history, and [docs/DECISIONS](docs/DECISIONS) for the architecture decision record behind every non-obvious choice along the way.
