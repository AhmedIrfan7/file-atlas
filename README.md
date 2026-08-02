# File Atlas

> The living map of everything on your computer.

**Status:** Pre-alpha. M0 (Foundation) in progress.

File Atlas is a local-first desktop application that indexes, understands, and helps you safely reshape everything stored on your machine. Think of it as the operating-system layer every organized person wishes already existed: a persistent memory of your files, a searchable map of your storage, and a trustworthy hand that helps you clean up without ever losing something you needed.

## Why

Existing tools fall into three weak buckets:

1. **File explorers** show you files but never understand them.
2. **Duplicate finders** run once and forget.
3. **Cleaners** are aggressive, unsafe, and full of dark patterns.

None of them build a durable, transparent understanding of what you own. File Atlas does.

## Principles

- **Local-first.** No file content leaves your machine unless you explicitly opt in.
- **Safety over speed.** Every destructive action is reversible or gated behind a clear confirmation. Recycle Bin, not `rm -rf`.
- **Transparency.** Every suggestion has a "why". No black-box automatic deletes.
- **Progressive disclosure.** Simple by default, deep on demand.
- **Cross-platform from day one.** Windows ships first, macOS and Linux follow.
- **Performance at real scale.** Designed for 2M+ files and 8TB+ drives.

## Roadmap

See [`docs/ROADMAP.md`](docs/ROADMAP.md) once M0 lands. Milestones:

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
- **M10** v1.0

## Stack

Rust core + React/TypeScript frontend, packaged with [Tauri](https://tauri.app). SQLite for the local index. See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Contributing

The project is currently maintained by a single developer while the core takes shape. Once the foundation is stable, external contributions will be welcomed via GitHub Issues and Pull Requests. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

[MIT](LICENSE)
