# Changelog

All notable changes to File Atlas will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it reaches v1.0.0.

## [Unreleased]

### Added

#### M0. Foundation

- MIT license, README, code of conduct, contributing guide, security policy
- Issue templates (bug report, feature request) and pull request template with safety checklist
- Architecture overview and roadmap (M0 through M10)
- Architecture Decision Record framework with ADR 0001 and ADR 0002 (Tauri + Rust core)
- Cargo workspace with 5 crates: `atlas-core`, `atlas-db`, `atlas-platform`, `atlas-search`, `atlas-recommender`
- pnpm workspace root
- Tauri v2 desktop shell with React + TypeScript, branded as "File Atlas"
- Tailwind CSS v4 with base design tokens
- Prettier, ESLint (flat config, typescript-eslint, react, hooks, refresh), rustfmt, clippy configs
- Cargo nextest configuration with default and CI profiles
- GitHub Actions workflows: `ci.yml` (rust + web), `build.yml` (Windows Tauri bundle), `codeql.yml`
- Dependabot for cargo, npm, github-actions
- Lefthook hooks: pre-commit (prettier, eslint, rustfmt), pre-push (typecheck, lint, format-check, clippy), commit-msg (commitlint)
- Commitlint enforcing Conventional Commits with File Atlas type list
- GitHub labels (type, area, priority, meta) and milestones (M0 through M10)

#### M1. Scanner and Index engine

- `atlas-db`: rusqlite connection with WAL mode, embedded migrations framework, migrations 0001 (schema) and 0002 (FTS5), row models (`FileRow`, `VolumeRow`, `ActionRow`), typed queries with idempotent upserts and rescan sweep helper
- `atlas-core`: `FileRecord` domain type, `SkipRules` with defaults for common build caches and Windows system paths, `scanner` module walking with cancellation and progress events, `indexer` batching scan events into SQLite with per-scan `scans` row lifecycle and `mark_removed_since` sweep
- `atlas-platform`: `PlatformFs` trait with `Volume` type, Windows implementation for `list_volumes`, `is_hidden`, `is_system` via the `windows` crate, stub for non-Windows platforms
- `atlas-cli`: `atlas` binary with `scan`, `stats`, `volumes`, and `search` commands; verified end-to-end scanning of the repo itself
- ADR 0003 (SQLite index with single-writer model), ADR 0004 (skip rules vs protected paths)
- 18 unit tests: connection open/migrate roundtrips, upsert idempotency, rescan removal sweep, scanner coverage, node_modules skip, cancellation, progress emission, Windows volume enumeration, indexer full-scan and rescan
