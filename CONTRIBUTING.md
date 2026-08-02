# Contributing to File Atlas

Thank you for considering a contribution. This document explains how the project is organized, what kinds of contributions are useful, and how to get changes accepted.

## Project status

File Atlas is early. The foundation and core engine are being built by a single maintainer. External Pull Requests are welcome for documentation, bug reports, tests, and small self-contained fixes. Larger changes should be discussed in an issue first, so that we can agree on shape before you invest time.

## Ground rules

1. **Be kind.** See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
2. **Assume a real user's data is on the line.** File Atlas touches files. Anything that could accidentally destroy user data is treated with maximum care. Every destructive path must go through the safety pipeline documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
3. **Local-first.** No feature may send file content or paths to a remote service without an explicit opt-in and a clear user-facing explanation.
4. **Cross-platform from day one.** Never introduce a Windows-only assumption into core code. Platform specifics go behind the `PlatformFs` trait.

## Development setup

You will need:

- Rust (stable) via [rustup](https://rustup.rs)
- Node.js 20 or newer
- pnpm 9 or newer
- Platform prerequisites for [Tauri](https://tauri.app/start/prerequisites/)

Then:

```bash
git clone https://github.com/AhmedIrfan7/file-atlas.git
cd file-atlas
pnpm install
cargo build --workspace
pnpm --filter desktop dev
```

## Branching

- `main` is protected. Every change lands via a Pull Request.
- Branch names are `feat/short-slug`, `fix/short-slug`, `docs/short-slug`, `chore/short-slug`, `refactor/short-slug`, `perf/short-slug`, `test/short-slug`.
- Keep branches small. If a branch grows past a few files or a couple hundred lines, split it.

## Commits

We follow [Conventional Commits](https://www.conventionalcommits.org). The subject line is a single sentence, imperative, under 72 characters. No em dashes.

Types in use:

- `feat` new user-facing capability
- `fix` bug fix
- `perf` performance improvement without behavior change
- `refactor` internal restructure without behavior change
- `docs` documentation only
- `test` tests only
- `chore` build, tooling, dependencies
- `ci` continuous integration
- `security` security fix or hardening
- `revert` revert a prior commit

Scope is optional but encouraged: `feat(scanner): stream entries from walkdir`.

Do not include `Co-Authored-By` trailers or AI attribution. Every commit is authored by the human contributor whose GitHub account made the change.

## Pull Requests

- Fill in the PR template.
- Add tests for any change to logic. UI changes need a screenshot or short recording.
- Update user-facing docs when behavior changes.
- CI must be green before merge.
- The maintainer will use a merge commit (not squash) to preserve fine-grained history.

## Reporting bugs

Open an issue using the Bug Report template. Include:

- OS and version
- File Atlas version (or commit hash)
- Steps to reproduce
- Expected vs actual behavior
- Any relevant logs from `%APPDATA%/FileAtlas/logs/` (Windows) or the equivalent on other platforms

## Reporting security issues

Please do not open a public issue. See [SECURITY.md](SECURITY.md).

## Code style

- Rust: `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- TypeScript: `pnpm lint` and `pnpm typecheck` must pass.
- Tests: `cargo nextest run` for Rust, `pnpm test` for JS.

## License

By contributing you agree that your contributions will be licensed under the [MIT License](LICENSE) of the project.
