# ADR 0003: SQLite index with a single-writer model

- Status: Accepted
- Date: 2026-08-03
- Deciders: AhmedIrfan7

## Context

File Atlas needs a durable local store for the file index. The store must:

- Survive process restarts and application updates
- Handle 2M+ rows without falling over
- Support both point queries (`find one file by path`) and analytical queries (`top 100 largest files by category`)
- Support full-text search over names and paths
- Ship inside a single-file installer with no external service

Options considered:

1. **SQLite with `rusqlite`.** Ubiquitous. Battle tested. FTS5 built in. Single file on disk. Rich ecosystem.
2. **`sled`.** Pure Rust key-value store. Fast writes. No SQL, no FTS. Would require us to build our own query layer.
3. **`redb`.** Modern, safe, embedded KV store. Same tradeoff as sled: no SQL, no FTS, more code to write.
4. **DuckDB.** Great for analytics but much larger binary and no built-in FTS.

## Decision

Use SQLite via `rusqlite` with the `bundled` feature. Journal mode is WAL. Synchronous is set to `NORMAL`. Foreign keys are enforced. Cache size is 64 MB.

Concurrency model:

- **Single writer.** All writes route through the `atlas-core::indexer` task, which owns the sole write connection. Business logic dispatches writes into this task through channels or awaited command handlers.
- **Many readers.** Read connections are obtained from a small pool. SQLite's WAL journal permits concurrent readers alongside a writer.
- Cross-thread coordination uses `crossbeam-channel` for the scanner-to-indexer path.

## Consequences

Positive:

- One-file store makes backup, portability, and reset trivial.
- FTS5 covers the M3 search milestone with no extra dependency.
- Bundled SQLite means the shipped binary has no libsqlite dependency to worry about.
- The single-writer rule sidesteps SQLite's "database is locked" errors under contention.

Negative:

- If the writer task falls behind, incoming scan events buffer up. We mitigate with a bounded channel between scanner and indexer.
- SQLite is not distributed. If we ever want multi-device sync (M9+), we will need a sync layer on top; SQLite is a fine source of truth for the local node.

## Alternatives revisited

If SQLite performance ever becomes a real bottleneck at 10M+ rows, the escape hatch is: keep the schema as-is but move to `rusqlite`'s `Connection::pragma_update` for `mmap_size` and `page_size` tuning, sharded by volume. A full store swap is not currently on the table.
