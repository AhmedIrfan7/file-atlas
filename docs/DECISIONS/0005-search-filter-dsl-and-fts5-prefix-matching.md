# ADR 0005: Search filter DSL and FTS5 prefix matching

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

M3 needs search that is both fast (sub-100ms on a warm cache, per the roadmap) and expressive enough to answer questions like "PDFs over 10MB I haven't touched in a year, somewhere under Downloads" without a separate advanced-search UI. Two design questions had to be settled:

1. What does the query text actually mean? A single input box has to support both free text and structured constraints.
2. How does free text get matched? SQLite FTS5 tokenizes on non-alphanumeric boundaries and does not support arbitrary substring search out of the box.

## Decision

### A small inline filter DSL, not a query builder UI

The single search box accepts a mix of free text and `key:value` / `key<op>value` tokens:

- `type:pdf` — extension equality
- `in:downloads` — folder substring
- `size>10mb`, `size<=1gb` — byte comparisons with unit suffixes (b/kb/mb/gb/tb)
- `age>1y`, `age<30d` — relative-age comparisons with unit suffixes (d/w/m/y)
- anything else is free text, matched against name and path
- double-quoted phrases (`"annual report"`) are kept together

This is implemented as three pure, independently testable stages in `atlas-search`: `parser` (text to `SearchQuery`), `planner` (query to parameterized SQL, no database touched), `runner` (executes against a connection). Keeping the planner DB-free means the SQL shape is unit tested without spinning up SQLite for every case.

We rejected a separate "advanced search" panel with dropdowns for filters. It would need its own state management, would not compose with free text as naturally, and would not give power users a shareable, memorizable syntax. The DSL is the kind of thing a user picks up from the placeholder text in the box and never needs a manual for.

### FTS5 prefix queries, not a trigram tokenizer

`files_fts` (added in M1, migration 0002) uses the default `unicode61` tokenizer, which splits on non-alphanumeric characters. This means it matches whole tokens, not arbitrary substrings. Typing "epor" will not find "report.pdf" because "epor" is not a token boundary match.

The planner works around this by turning each free-text word into a quoted prefix query: `resume` becomes `"resume"*`, which matches any token starting with "resume". This covers the overwhelmingly common case (typing the start of a filename or word) without adding a trigram virtual table, which would roughly triple the FTS index's on-disk size for a benefit ("bat" matching "combat") that most users never ask for.

Quoting each word before appending `*` is also the sanitization strategy: FTS5's query syntax treats `:`, `-`, `(`, `)` and other characters as operators, so a filename fragment containing them would otherwise be misinterpreted as a malformed or dangerous query. Quoting forces literal interpretation.

## Consequences

Positive:

- The DSL and free text share one input, matching how people actually think about search ("give me PDFs" is one thought, not two separate steps).
- Parser and planner have zero I/O, so their unit tests run in microseconds and cover every filter kind and combination without touching disk.
- FTS5 prefix matching is a one-line change (`"word"*`) with no schema migration and no extra storage.

Negative / accepted limitations:

- Mid-word substring search does not work (searching "port" will not find "report.pdf" via FTS; it would need to appear at a token start). This is a known, accepted gap. If it becomes a real complaint, the fix is an FTS5 trigram tokenizer on a new virtual table, additive and reversible, not a rewrite.
- Age units (month = 30 days, year = 365 days) are approximations, not calendar-aware. Documented in the parser's module comment; acceptable for "roughly how old" filtering.

## Related work

- `crates/atlas-search/src/parser.rs`, `planner.rs`, `runner.rs`, `saved.rs`
- Migration `0002_fts.sql` (FTS5 table, M1) and `0003_saved_searches.sql` (M3)
- ADR 0003 (SQLite single-writer model) — search reads share the same mutex-guarded connection as everything else
