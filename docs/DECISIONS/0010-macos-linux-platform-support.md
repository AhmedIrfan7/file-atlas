# ADR 0010: macOS and Linux platform support

- Status: Accepted
- Date: 2026-08-04
- Deciders: AhmedIrfan7

## Context

M8's roadmap goal is parity with Windows on macOS and Linux: real `atlas-platform` implementations, trash integration, shell integration, and an expanded CI build matrix. Reading the existing code before writing anything turned up two problems bigger than "write two new files":

1. `atlas_core::safety::DEFAULT_PROTECTED_PREFIXES` and `atlas_core::skip_rules::SkipRules::default()` were hardcoded to Windows paths (`C:\Windows`, `C:\Program Files`, ...). On macOS or Linux, neither list ever matched anything, meaning the delete guardrail and the scanner's system-path skip list were silent no-ops on those platforms. This is a safety regression waiting to happen, not a cosmetic gap, and had to be fixed as part of this milestone rather than filed as a follow-up.
2. The `trash` crate's `os_limited` module (`list()`, `restore_all()`), which the existing `restore_from_trash` design depends on to find a previously-trashed item again, is not available on macOS at all. This was only discovered by actually cross-compiling `atlas-platform` for `x86_64-apple-darwin`, which is the first time this exact code path was checked against a real macOS target since M4 wrote it.

## Decision: per-OS default path lists, not one shared list

`DEFAULT_PROTECTED_PREFIXES` (`atlas-core/src/safety.rs`) and `default_system_prefixes()` (`atlas-core/src/skip_rules.rs`) are now `#[cfg]`-gated per OS: Windows keeps its existing list, macOS gets `/System`, `/Library`, `/private`, `/usr`, `/bin`, `/sbin`, and `/Applications` (installed applications, matching Windows' own inclusion of Program Files), Linux gets `/usr`, `/bin`, `/sbin`, `/lib`, `/lib64`, `/etc`, `/boot`, `/proc`, `/sys`, `/dev`, `/opt`, and `/snap` (installed applications and packages, the same reasoning as `/Applications`). Note that this protected-path list is not the same list `is_system` checks against below; `/Applications` is deliberately in one and not the other, because the two mechanisms answer different questions (see the `is_system` section).

`seed_defaults` also now resolves the current user's real trash folder at runtime (`~/.Trash` on macOS, `~/.local/share/Trash` on Linux, via the already-a-dependency `dirs` crate) and seeds it alongside the compile-time list. This could not be a `const` list entry since the home directory is not known until runtime; Windows does not need this because its trash is the single machine-wide `C:\$Recycle.Bin`, already a static path.

Prefix matching (`path_has_prefix` in both files) is now case-insensitive on Windows and macOS (both case-insensitive-by-default filesystems) but case-sensitive on Linux, where `/USR` and `/usr` are genuinely different paths. This was already latently wrong before M8 (the old `starts_with_ci` was unconditionally case-insensitive), it just had no observable effect while every prefix was a Windows-only path.

## Decision: `is_system` is prefix-based on macOS/Linux, not attribute-based

Windows' `is_system` reads the real `FILE_ATTRIBUTE_SYSTEM` bit from the filesystem: a genuine per-file OS attribute. Unix filesystems (ext4, APFS, etc.) have no equivalent per-file "system" attribute, so `is_system` on macOS and Linux is answered by known-prefix membership instead (`/System`, `/usr`, `/proc`, and similar). This is a different mechanism from `safety`'s protected-path list, not a duplicate of it: `is_system` (currently unused outside its own tests; a future "show system files" toggle is the natural caller) answers "is this the kind of path an OS-attribute check would flag", while `protected_paths` answers "must a delete refuse to touch this". `/Applications` is genuinely not FILE_ATTRIBUTE_SYSTEM-equivalent (installed apps are not OS-internal files), so it is excluded from `is_system`'s prefix list on macOS even though it is protected against deletion.

## Decision: `is_hidden` uses each platform's real convention

- Windows: the `FILE_ATTRIBUTE_HIDDEN` bit (unchanged from M1).
- macOS: the dot-file naming convention (`.foo`), OR the `UF_HIDDEN` `st_flags` bit that Finder sets via `chflags hidden` independently of naming. Both are real, commonly-hit cases; checking only one would miss files hidden the other way.
- Linux: the dot-file naming convention only. Standard desktop file managers (Nautilus, Dolphin, Thunar) all use this convention; there is no widely-supported separate hidden-attribute bit in common Linux filesystem use the way Windows and macOS have. Some file managers also honor a per-directory `.hidden` list file (an additional opt-in hiding mechanism); reading and parsing that file on every `is_hidden` call is a real feature but a different one, and is not built here.

## Decision: volume enumeration

- macOS: enumerate `/Volumes` (each entry is a mounted volume), plus always ensure the boot volume (`/`) is represented. Device-id comparison (`std::os::unix::fs::MetadataExt::dev()`, portable and part of std, no FFI needed) deduplicates the case where `/Volumes/Macintosh HD` (or whatever the boot volume is named) is the same device as `/`, so it is not double-counted. Capacity and filesystem-type name come from one `libc::statfs` call (macOS-specific, exposes `f_fstypename` directly; POSIX `statvfs` does not carry a type name).
- Linux: parse `/proc/mounts` (a plain-text table the kernel always maintains, no `libudev` or similar dependency needed) and filter out a fixed list of pseudo/virtual filesystem types (`proc`, `sysfs`, `tmpfs`, `overlay`, `squashfs`, and about twenty others) so container and kernel bookkeeping mounts never appear as "volumes" a user would recognize. Capacity comes from `libc::statvfs` per real mount point. Mount paths are unescaped from `/proc/mounts`'s octal escaping (`\040` for a space) since the raw field is not usable as a path otherwise.

Both use `libc` (a new dependency, target-scoped to `cfg(any(target_os = "macos", target_os = "linux"))`) only for the one syscall each platform actually needs; everywhere else favors std APIs (`MetadataExt::dev()`, `MetadataExt::st_flags()` on macOS) over raw FFI.

## Decision: shell integration ("reveal in file manager") on all three platforms

`open_in_file_manager` had zero implementations and zero callers before this milestone, on any platform, despite existing in the trait since M1. It is now implemented everywhere and wired to a real "Show in folder" button on search results:

- Windows: `explorer.exe /select,<path>` (spawned, not waited on; Explorer's own exit codes are not a reliable success signal).
- macOS: `open -R <path>` (reveals and selects in Finder).
- Linux: `xdg-open <parent-folder>`. There is no cross-desktop-environment equivalent of Explorer's `/select,` or Finder's `-R` on Linux (no universal "reveal and select this exact file" verb), so the portable substitute is opening the containing folder, which every `xdg-open`-compatible file manager understands.

## Decision: `restore_from_trash` reports `Unsupported` on macOS, not a hand-rolled fallback

`send_to_trash` uses `trash::delete()` on all three platforms; this is the same well-tested crate code path everywhere and carries no extra risk on macOS. The problem is specific to restore: `trash::os_limited` (`list()`, `restore_all()`), which Windows and Linux both use to find a previously-trashed item again by identity, is gated out of the crate entirely on macOS (`cfg(any(target_os = "windows", all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))))`, confirmed directly against the crate source). There is no crate-provided way to enumerate or restore a specific item from the macOS Trash.

Two ways to close this gap were considered:

1. Hand-roll a macOS-specific restore using `NSFileManager.trashItemAtURL(_:resultingItemURL:)` directly via `objc2`/`objc2-foundation` (both already present as transitive dependencies of the `trash` crate's own macOS backend). This API genuinely does return the exact URL the item was moved to, which would make restore trivial (`fs::rename` back to the original path) without needing `os_limited` at all.
2. Report `Unsupported` from `restore_from_trash` on macOS and leave `send_to_trash` as the only implemented half.

Option 2 was chosen. This project has no macOS machine to test against, in this environment or otherwise; the only way to verify new Objective-C FFI code before it ships would be trusting it blind until a real user's Mac either works or does not. For a safety-critical code path (trashing and restoring a user's real files), shipping unverified ObjC bridging code is a worse risk than an honest capability gap. The operation itself stays completely safe either way: `send_to_trash` on macOS moves the file into the real Trash, and the user retains full manual "Put Back" ability through Finder regardless of whether File Atlas's own restore button works. `atlas_core::actions::restore_action` already turns a platform `Unsupported` error into a clean `RestoreOutcome { ok: false, reason: Some(..) }` rather than panicking, so this degrades exactly the way the architecture was designed to handle a platform capability gap. Revisit this once either the `trash` crate adds macOS support upstream, or a contributor with real macOS hardware can validate a direct `NSFileManager` implementation.

## Decision: how this was verified without a Mac or a Linux machine

This project is developed entirely on Windows. Two verification paths were used instead of the computer-use, real-app-launch pattern every prior milestone used:

1. **Local cross-compilation.** `rustup target add x86_64-unknown-linux-gnu x86_64-apple-darwin`, then `cargo check`/`cargo clippy --target <triple>` against `atlas-platform` directly. This works because `cargo check`/`clippy` only need to type-check and lint, not link, and `atlas-platform`'s dependencies are all pure Rust (`thiserror`, `serde`, `tracing`, `trash`, `libc`) with no C build step. This is exactly how the `trash::os_limited` gap on macOS was actually found: the code compiled fine by inspection, but the cross-target `cargo check` failed with a real `E0433`. `atlas-core` could not be cross-checked the same way: it depends on `atlas-db`, which depends on `rusqlite`'s `bundled` feature, which compiles SQLite's C source and needs a real cross C toolchain (`x86_64-linux-gnu-gcc` or equivalent) this machine does not have.
2. **CI as the authoritative test.** `ci.yml`'s `rust` job and `build.yml`'s `desktop` job both now run on `windows-latest`, `macos-latest`, and `ubuntu-latest`. These are real machines with real C toolchains, so this is where `atlas-core`'s per-OS `safety`/`skip_rules` tests, and every `atlas-platform` unit test (including the new `macos_impl`/`linux_impl` suites), actually execute for the first time. Treat a green run on all three OSes as the real completion signal for this milestone, the same weight a real-app computer-use verification carried in M2 through M7.

## Consequences

Positive:

- The delete guardrail and scan-skip list are no longer silent no-ops on macOS and Linux; this was a latent safety gap this milestone found and fixed, not a new feature.
- `atlas-platform`'s pure-Rust dependency graph made real cross-target verification possible from a Windows-only development machine; this is a direct payoff of ADR 0002's platform-isolation design.
- The `trash::os_limited` macOS gap was caught before merging, not after a user reported broken restore on a Mac.

Negative / accepted limitations:

- Restoring a trashed item through File Atlas's own UI does not work on macOS yet. The item is not at risk (Finder's "Put Back" still works), but the app's own affordance for it does not.
- Linux's `is_hidden` does not honor per-directory `.hidden` list files, only the dot-file convention.
- Linux's `open_in_file_manager` opens the containing folder rather than selecting the exact file, since no portable "select this file" verb exists across Linux file managers.
- None of the macOS or Linux code paths have been run on a real machine by a human yet; correctness rests on cross-compilation type-checking plus whatever the CI matrix reports on its first real run.

## Related work

- `crates/atlas-core/src/safety.rs`, `crates/atlas-core/src/skip_rules.rs`
- `crates/atlas-platform/src/{macos_impl,linux_impl,windows_impl,trash_common}.rs`
- `apps/desktop/src-tauri/src/commands.rs` (`open_in_file_manager`)
- `.github/workflows/{ci,build}.yml`
- ADR 0002 (Tauri/Rust core, and the `PlatformFs` trait boundary this milestone depends on)
- ADR 0006 (why trash/restore uses the `trash` crate rather than hand-rolled platform APIs, the same reasoning that rules out a hand-rolled macOS restore fallback here)
