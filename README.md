# File Atlas

> The living map of everything on your computer.

**Status:** v1.0.0. Windows, macOS, and Linux.

File Atlas is a local-first desktop application that indexes, understands, and helps you safely reshape everything stored on your machine. Think of it as the operating-system layer every organized person wishes already existed: a persistent memory of your files, a searchable map of your storage, and a trustworthy hand that helps you clean up without ever losing something you needed.

[Download the latest release](https://github.com/AhmedIrfan7/file-atlas/releases/latest) &middot; [Release notes](RELEASE_NOTES.md) &middot; [Changelog](CHANGELOG.md)

## Why

Existing tools fall into three weak buckets:

1. **File explorers** show you files but never understand them.
2. **Duplicate finders** run once and forget.
3. **Cleaners** are aggressive, unsafe, and full of dark patterns.

None of them build a durable, transparent understanding of what you own. File Atlas does.

## Features

- **Home.** Understand your disk within seconds: category breakdown, largest and oldest files, and a "not touched in over a year" bucket.
- **Search.** Full-text and structured search under 100ms on a warm cache. Filters like `type:pdf`, `size>10mb`, `age>1y`, saved searches.
- **Duplicates.** BLAKE3-hashed duplicate detection with a safe default (keep newest, show why) and bulk trash with preview and full undo.
- **Smart Cleanup.** Rule-based recommendations for empty folders, forgotten installers, old archives, and screenshot pileups. Every suggestion shows its reason and confidence, no black boxes.
- **Storage Map.** A treemap of where your space actually went, with drill-down by folder and category, plus a time filter.
- **Life Timeline.** A chronological view of your digital life: project bursts, screenshot bursts, and semester periods, auto-grouped.
- **Local AI Search.** Natural-language search over your files, backed by a local embedding index and an optional local LLM via [Ollama](https://ollama.com), entirely on-device by default. A cloud LLM path exists too, but it is opt-in, clearly labeled, and confirmed per request, never silently enabled.
- **Autoupdate.** Checks for new releases on launch and shows what's available. Installing one always takes an explicit click, never automatic.

## Principles

- **Local-first.** No file content leaves your machine unless you explicitly opt in.
- **Safety over speed.** Every destructive action is reversible or gated behind a clear confirmation. Recycle Bin, not `rm -rf`.
- **Transparency.** Every suggestion has a "why". No black-box automatic deletes.
- **Progressive disclosure.** Simple by default, deep on demand.
- **Cross-platform.** Windows, macOS, and Linux, with real per-OS implementations, not stubs.
- **Performance at real scale.** Designed for 2M+ files and 8TB+ drives.

## Roadmap

All ten planned milestones have shipped. See [`docs/ROADMAP.md`](docs/ROADMAP.md) for each milestone's goal and [`CHANGELOG.md`](CHANGELOG.md) for what was actually built:

- **M0** Foundation (repo, tooling, CI)
- **M1** Scanner and index engine
- **M2** Home view
- **M3** Search
- **M4** Duplicates
- **M5** Smart cleanup with explainable recommendations
- **M6** Storage map
- **M7** Life timeline
- **M8** macOS and Linux support
- **M9** Local AI layer
- **M10** v1.0 release

## Stack

Rust core + React/TypeScript frontend, packaged with [Tauri](https://tauri.app). SQLite for the local index. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full design, and [`docs/DECISIONS`](docs/DECISIONS) for the architecture decision record behind every non-obvious choice.

## Known limitations

- Installers are not yet signed with a paid OS-level publisher certificate, so Windows and macOS may show an unknown-publisher warning on first install. The updater's own cryptographic signature is unaffected by this and still verifies every update independently. See [ADR 0012](docs/DECISIONS/0012-v1-release-pipeline.md).
- Restoring a trashed item through File Atlas's own UI is not available on macOS yet (Finder's own "Put Back" still works). See [ADR 0010](docs/DECISIONS/0010-macos-linux-platform-support.md).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for development setup, branching, commit conventions, and how to open a Pull Request.

## License

[MIT](LICENSE)
