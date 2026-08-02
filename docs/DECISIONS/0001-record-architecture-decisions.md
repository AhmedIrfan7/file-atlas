# ADR 0001: Record architecture decisions

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

Architectural choices are easier to make than they are to remember. Six months from now, the reason we picked SQLite over an embedded key-value store or picked Tauri over Electron will be lost unless we write it down.

We need a lightweight, human-readable record of every significant decision so that:

- Future contributors understand why the project looks the way it does
- We can revisit decisions when the context changes
- We can push back on suggestions that were considered and rejected

## Decision

Use Architecture Decision Records (ADRs) in the style described by Michael Nygard.

- Every ADR lives in `docs/DECISIONS/`
- Filenames follow the pattern `NNNN-short-slug.md` where `NNNN` is a zero-padded sequence starting at `0001`
- Each ADR has these sections: Status, Date, Deciders, Context, Decision, Consequences
- Statuses are one of: Proposed, Accepted, Deprecated, Superseded by ADR-XXXX
- When an ADR is superseded, we do not delete it. We change its status and add a link to the successor.

## Consequences

Positive:

- Future decisions are easier because we can see the shape of prior thought.
- New contributors have a fast onboarding path into the "why" of the codebase.
- Bad ideas we already rejected can be pointed at directly.

Negative:

- Small discipline cost. Every architectural change becomes two artifacts: the code and the ADR.
- Risk of stale ADRs if we forget to mark them Superseded.

Mitigation: the pull request template asks whether an ADR change is needed.
