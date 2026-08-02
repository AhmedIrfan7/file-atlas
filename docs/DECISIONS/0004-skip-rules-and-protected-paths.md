# ADR 0004: Skip rules and protected paths

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

The scanner must be fast enough to walk a million-file drive in minutes, not hours, and it must never expose the user to danger by walking or classifying content in system-owned locations. Two policies solve two different problems:

- **Skip rules** are performance guards. They prevent the walker from descending into directories whose contents are almost never worth indexing (`node_modules`, `.git`, `target`, `__pycache__`, `.venv`, and so on) and out of paths where nothing user-actionable lives (`C:\Windows`, `C:\Program Files`).
- **Protected paths** are safety guards. They prevent the actions layer from moving, renaming, or trashing files inside system-critical or user-designated safe zones, even if the index contains them.

The two policies overlap in intent but not in enforcement. A path may be scanned but still protected (say, the user's Documents root). A path may be skipped but not protected in the schema sense because we never index it in the first place.

## Decision

**Skip rules** live in `atlas-core::skip_rules::SkipRules`. Defaults ship with:

- Directory names skipped by name: `.git`, `.hg`, `.svn`, `node_modules`, `__pycache__`, `.venv`, `.mypy_cache`, `.pytest_cache`, `.tox`, `target`, `build`, `dist`, `.next`, `.nuxt`, `.turbo`, `.cache`, `.gradle`, `.idea`, `.vscode`, `$RECYCLE.BIN`, `System Volume Information`
- Case-insensitive path prefixes skipped: `C:\Windows`, `C:\Program Files`, `C:\Program Files (x86)`, `C:\ProgramData`, `C:\$Recycle.Bin`, `C:\System Volume Information`
- Hidden files skipped by default (leading dot on POSIX; Windows attribute check arrives when `atlas-platform` is fully wired into the scanner)
- Symlinks not followed

Users can add custom directory names and path prefixes. Users can opt in to hidden files.

**Protected paths** live in the `protected_paths` table (introduced in migration 0001) and, at runtime, are consulted by the safety module before any destructive operation. Seed rows are inserted on first run: system directories, the user profile roots (never the leaves), and any explicit user-selected safe zones from the onboarding wizard.

The scanner never consults `protected_paths`. The actions layer never consults `SkipRules`. Each policy has one job.

## Consequences

Positive:

- A user who deliberately points the scanner at `C:\Windows` gets nothing back, which is the correct default; they can override this with a permissive `SkipRules`.
- A user who has `C:\Users\me\Documents` indexed still cannot accidentally trash it because it is a protected path.
- The two data structures are simple and can be edited in the UI without touching each other.

Negative:

- The overlap can confuse new contributors. The module docs and this ADR are the mitigation.
- Every new user-facing operation must remember to consult `protected_paths`. A guardrails helper in `atlas-core::safety` is the single point of enforcement.

## Related work

- Migration `0001_initial_schema.sql` creates `protected_paths`.
- `atlas-core::skip_rules::SkipRules` implements the walker rules.
- `atlas-core::safety` (M4) will implement the guardrails pipeline.
