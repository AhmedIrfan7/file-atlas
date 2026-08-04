# ADR 0012: v1.0 release pipeline

- Status: Accepted
- Date: 2026-08-04
- Deciders: AhmedIrfan7

## Context

M10's roadmap goal is "a shippable public release": autoupdater, signed installers, landing page, screenshots/demo video, release notes, and the `v1.0.0` tag itself. This ADR covers the decisions specific to turning the app into something that updates itself and gets published, plus the parts of that goal that could not be done for real in this environment and had to be scoped down honestly instead of faked.

## Decision: Tauri's built-in updater, minisign-signed, self-hosted via GitHub Releases

`tauri-plugin-updater` + `tauri-plugin-process` were added rather than a custom update-check mechanism. The plugin checks a `latest.json` manifest at a fixed URL, compares versions, and if newer, downloads and verifies a minisign signature before installing. `tauri.conf.json` points `plugins.updater.endpoints` at `https://github.com/AhmedIrfan7/file-atlas/releases/latest/download/latest.json`, so GitHub Releases doubles as the update host; no separate update server to run or pay for.

The signing keypair was generated for real via `pnpm tauri signer generate --ci -w <path>`. The `--ci` flag skips the interactive password prompt, producing a passwordless private key. This is standard practice for CI-held signing keys: the key only ever lives as a GitHub Actions secret (`TAURI_SIGNING_PRIVATE_KEY`), and the secret itself, not a password, is what protects it. The public half is embedded directly in `tauri.conf.json` (`plugins.updater.pubkey`), which is safe to publish since it can only verify signatures, not create them.

`UpdateChecker.tsx` checks on launch and shows an "Install & restart" button rather than silently auto-installing. The user always sees what version is available and clicks to proceed; nothing downloads or restarts the app without that click.

## Decision: real OS code-signing is deferred, not faked

Tauri's updater signature (above) proves an update came from this project's CI and was not tampered with in transit. It does **not** stop Windows SmartScreen or macOS Gatekeeper from warning that the installer is from an "unknown publisher" on first run, since that requires a completely different mechanism: a paid Authenticode certificate (Windows) or an Apple Developer ID plus notarization (macOS), each tied to a verified real-world identity.

I cannot obtain either of these on the user's behalf: both require a human to complete identity verification and pay for a certificate/developer account. Rather than skip this silently or fabricate a certificate reference that would fail in CI, it is documented here as a known, deferred gap. `release.yml` ships unsigned-at-the-OS-level installers; users will see an unknown-publisher warning until real certificates are purchased and wired into the workflow's `TAURI_SIGNING_PRIVATE_KEY`-style secrets (Tauri/`tauri-action` support both once available).

## Decision: draft releases, not auto-publish

`release.yml` passes `releaseDraft: true` to `tauri-apps/tauri-action`. A tag push builds, signs (updater signature), and stages a GitHub Release with all installers and `latest.json` attached, but the release stays a draft until a human clicks Publish in the GitHub UI. Publishing a public release is a hard-to-reverse, highly visible action; staging it as a reviewable draft matches how this project has treated every other irreversible action all along (destructive-operation confirmations, force-push refusals, and so on). The alternative, publishing immediately on tag push, would mean a typo'd tag or a bad build goes live with no review step.

## Decision: `build.yml` and `release.yml` stay separate workflows

`build.yml` runs on every push/PR to `main` and answers "does the bundle still build"; it has no side effects and no secrets. `release.yml` runs only on a `v*` tag push (or manual dispatch) and is the only workflow that signs and publishes. Splitting them means the frequent, cheap check never touches signing secrets, and the expensive three-OS signed build only ever runs when a release is actually intended.

## Decision: no demo video, real static screenshots instead

`ffmpeg` is not available in this environment (`ffmpeg -version` and `where ffmpeg` both fail) and there is no screen-recording tool available either. Rather than describe a video that does not exist, the roadmap item is scoped down to real static screenshots captured via computer-use against the actual running app, covering each major view (Home, Search, Duplicates, Cleanup, Storage, Timeline, AI Search). A demo video remains a good follow-up once this runs somewhere with recording capability.

## Consequences

Positive:

- The update mechanism is real end-to-end: a real keypair, a real plugin, a real manifest host, verified against the actual running app.
- Nothing about the release pipeline can silently auto-publish or auto-install without a human action at the point that matters (draft review, install click).
- The gap between "signed update payload" and "signed installer trusted by the OS" is documented rather than glossed over, so a future contributor with a certificate knows exactly what to wire in and where.

Negative / accepted limitations:

- Users installing File Atlas for the first time (or updating on Windows via the raw installer rather than the in-app updater) will see an unknown-publisher warning until real code-signing certificates are purchased.
- No demo video ships with v1.0.0; static screenshots substitute.
- The passwordless signing key means anyone with read access to the `TAURI_SIGNING_PRIVATE_KEY` GitHub secret can sign a malicious update; this is the same trust boundary every CI-secret-based signing setup accepts, scoped to this repository's collaborator list (currently just AhmedIrfan7).

## Related work

- `apps/desktop/src-tauri/tauri.conf.json` (`plugins.updater`), `apps/desktop/src-tauri/capabilities/default.json`
- `apps/desktop/src/components/UpdateChecker.tsx`, `apps/desktop/vite.config.ts`
- `.github/workflows/build.yml`, `.github/workflows/release.yml`
- ADR 0010 (the CI-as-verification pattern this ADR reuses for the three-OS release matrix)
