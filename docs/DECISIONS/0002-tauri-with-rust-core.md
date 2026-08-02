# ADR 0002: Use Tauri with a Rust core

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

File Atlas needs a desktop shell and a heavy-duty core. The core must:

- Walk filesystems containing millions of entries
- Hash hundreds of gigabytes for duplicate detection
- Perform file operations safely (move, rename, trash)
- Ship the same code on Windows, macOS, and Linux
- Produce a small, trustworthy installer

We considered four families of stacks:

1. **Electron + Node.js.** Familiar, but the process runtime is heavy (100 MB+ installers), and Node's filesystem throughput does not compete with a native scanner on large trees. Memory-safety guarantees for destructive operations are also weaker.
2. **Tauri + Rust core + web frontend.** Small binaries (sub-10 MB), Rust's memory safety and throughput, cross-platform out of the box, and a web frontend keeps UI iteration fast.
3. **Native per-platform (WinUI 3, SwiftUI, GTK).** Best per-platform experience but triples the maintenance surface, incompatible with a solo maintainer at the start.
4. **Flutter Desktop.** Reasonable choice but the ecosystem for low-level filesystem work is thin compared to Rust.

## Decision

Adopt **Tauri** as the desktop shell with a **Rust workspace** for the core and a **React + TypeScript** frontend.

- The Rust workspace lives under `crates/`
- The Tauri app lives under `apps/desktop/`
- The frontend is Vite + React + TypeScript, using Tailwind for styling
- Platform-specific code is isolated behind the `PlatformFs` trait in `atlas-platform`
- Rust code communicates with the frontend only through Tauri commands and events

## Consequences

Positive:

- Sub-10 MB installer
- Strong safety story for destructive operations
- Cross-platform from day one at no extra cost
- Rust ecosystem for filesystem work (`ignore`, `walkdir`, `blake3`, `rusqlite`) is best in class

Negative:

- The sole maintainer's primary language experience is Python and JavaScript. Rust has a learning curve.
- Two toolchains to install and keep in sync in CI.

Mitigations:

- Core Rust crates are documented with plain-English module headers.
- UI-only work never requires touching Rust.
- CI installs both toolchains and caches them aggressively.

## Alternatives

If Tauri becomes untenable (for example a critical unsupported platform feature), we can migrate to Electron while keeping the Rust core as an out-of-process binary invoked via IPC or a `napi` binding. The `atlas-core` crate is designed to be shell-agnostic.
