## Summary

<!-- One or two sentences: what does this PR do and why. -->

## Changes

<!-- Bullet list of the meaningful changes. -->

-
-

## Screenshots or recordings

<!-- Required for UI changes. Delete this section for pure backend or docs PRs. -->

## Safety checklist

<!-- Required for any change that reads, writes, moves, or deletes files. Delete this section if not applicable. -->

- [ ] All destructive operations route through the safety pipeline (guardrails, preview, confirm, action log)
- [ ] No `#[cfg(target_os)]` conditionals leaked into core modules
- [ ] Protected paths list is respected
- [ ] Behavior is idempotent or explicitly documented as not
- [ ] Failure modes are logged and surfaced to the user

## Testing

- [ ] Unit tests added or updated
- [ ] Integration tests added or updated
- [ ] Manual smoke test performed on Windows
- [ ] Manual smoke test performed on macOS (if applicable to this change)
- [ ] Manual smoke test performed on Linux (if applicable to this change)

## Documentation

- [ ] User-facing docs updated (`docs/USER_GUIDE.md`, README)
- [ ] Architecture docs updated (`docs/ARCHITECTURE.md`)
- [ ] Decision record added under `docs/DECISIONS/` (for architectural changes)
- [ ] Changelog entry added (or the change is trivial)

## Linked issues

Closes #
Related to #
