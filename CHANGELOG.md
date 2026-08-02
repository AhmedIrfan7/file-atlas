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
