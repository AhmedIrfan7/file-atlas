# ADR 0011: Local AI layer scope

- Status: Accepted
- Date: 2026-08-04
- Deciders: AhmedIrfan7

## Context

M9's roadmap goal is natural-language search and smart suggestions "without sending data off-device by default": a local embedding index, natural-language query to SQL translation, optional local LLM integration via Ollama, and an optional, clearly-labeled, per-request-confirmed cloud path. Several scoping questions had to be settled before writing code, and one real bug was found mid-implementation that changed the shape of the Tauri command layer.

## Decision: Ollama only, no bundled local model runtime

The roadmap sketch already named Ollama specifically, which settles the biggest question: this milestone does not bundle model weights or a Rust ML runtime (`candle`, `ort`). `atlas-ai` talks to Ollama's HTTP API (`http://localhost:11434`) for both embeddings and chat, and degrades to a clear `Unavailable`/`NoChatModel` error rather than blocking anything when Ollama is not running or a model is not installed. This was directly testable: Ollama was already running on the development machine with `nomic-embed-text` pulled, and `llama3.2:1b` (about 1.3 GB) was pulled with the user's explicit permission specifically to verify the translation path against a real small model rather than a scripted fake.

## Decision: translate to the existing filter DSL, never generate SQL

"Natural-language query to SQL translation" as literally described would mean an LLM writing raw SQL that then executes against the real index. For a project that treats guardrails and safe defaults as architectural priority number one (see ADR 0004, ADR 0006), handing an LLM's output straight to a SQL execution path is a real injection surface, not a hypothetical one. Instead, `atlas_ai::query_translation::translate` turns natural language into `atlas_search`'s existing filter DSL string (`type:pdf size>10mb age<1y`), which then runs through the exact same parser -> planner -> runner pipeline M3 already built and tested. The LLM's output is never trusted blind: it is parsed with the real `atlas_search::parser::parse`, and any input that fails to parse falls back to being treated as plain free text rather than erroring out. This means a broken, absent, or confused translator degrades search quality; it never breaks search or introduces a new execution surface.

One real failure mode this validation step exists for, found by reasoning through what parser success alone cannot catch: an LLM asked to translate "documents" quite naturally reaches for `type:document`, which parses as a syntactically valid filter (the parser just lowercases whatever follows `type:`) but can never match a real file, since `type:` compares against file extension and no file has the literal extension "document". `translate` additionally checks parsed `Extension` filters against the classifier's category name list (`Image`, `Video`, `Audio`, `Document`, `Archive`, `Installer`, `Code`, `Other`) and falls back to free text if they collide. Confirmed as a real behavior, not a hypothetical, by a unit test built specifically to catch it.

Running the real local model end to end during verification (`llama3.2:1b`, not a fake) surfaced further findings worth recording, in increasing order of severity:

- It does not always pick the technically best filter primitive. Asked to translate "large pdf files bigger than 10 megabytes", it correctly produced a size filter but left "pdf" as free text rather than an explicit `type:pdf` filter. Both are valid, working translations (free text still matches file names), just not identical ones; the fallback/validation logic accepts either, since its job is "reject clearly wrong output," not "enforce one canonical phrasing."
- It wrote `type:zip, size>10mb` with a comma-space separator between filters, unprompted. `"type:zip,"` (with the trailing comma) parses "successfully" as the literal, never-matching extension `"zip,"` rather than erroring, so this would have silently shipped a broken filter as if it were a valid one — a real gap the syntactic-validity check alone could not have caught. `clean()` now strips commas outside quoted phrases before parsing.
- Fixing that surfaced a second, independent bug in the same function: `clean()`'s quote-unwrapping used `str::trim_matches('"')`, which strips matching characters from each end of a string independently. A response that starts with a quoted phrase but does not end with one (`"Smith, John" type:pdf`) is not "wrapped" in the sense the code meant, but `trim_matches` stripped its opening quote anyway, breaking the phrase's pairing. Replaced with an explicit starts-with-and-ends-with check that only unwraps a genuinely fully-wrapped response.
- Across repeated runs of the identical feature, it sometimes invented age or folder filters nobody asked for (`"pdf files"` becoming `type:pdf in:*file* age>6m`), and did so inconsistently — the same request could translate cleanly on one run and produce implausible syntax on another. The system prompt was tightened to explicitly forbid adding a filter the request does not specifically imply, with worked examples; this reduced but did not eliminate the behavior.

The last finding does not have a code fix, and none is being pursued: it is a real, disclosed capability limit of a 1-billion-parameter local model doing zero-shot structured generation, not a defect in the translation pipeline. The pipeline's job is to make every outcome safe (parseable and not obviously wrong, or a transparent fallback to free text) and visible (the UI always shows the translated query before running it); it was never meant to guarantee this specific small model's output quality, which the docs should not overstate.

## Decision: embeddings are scoped to file names, not extracted file contents

"A local embedding index" could mean indexing extracted text from inside PDF/DOCX files. That is deferred, not built here: reading and extracting text from arbitrary documents is a real feature with its own failure modes (encoding, scanned-image PDFs with no text layer at all, files large enough to matter for indexing time), and half-building it inside this milestone would mean shipping something that silently under-delivers on a name like "semantic search of your documents." What ships is embeddings over a lightly-enriched file name string (name, category, containing folder — e.g. `"invoice.pdf (Document) in folder Taxes"`), which is real, useful signal on its own (verified directly against a real Ollama instance) and is a clean foundation to layer extracted-content embeddings onto later without changing the storage shape (`file_embeddings` already keys by path and model, independent of what text was embedded).

Similarity search is a linear scan over stored vectors computed in Rust, not a SQLite vector extension (`sqlite-vec` or similar): at the scale this targets (tens of thousands of files), a full scan is fast enough, and it avoids a new runtime dependency for approximate nearest-neighbor indexing before anyone has hit a real performance ceiling. Same reasoning as ADR 0008's choice of on-demand aggregation over a maintained rollup for the storage map.

## Decision: cloud AI is an OpenAI-compatible client, off by default, confirmed per request

`CloudProvider` speaks the `/chat/completions` shape rather than one named vendor's SDK, since that shape is now a de facto standard many providers and self-hosted gateways speak; a configurable base URL covers all of them without locking into one vendor before this project has committed to one. `cloud_enabled` defaults to `false` in `AiSettings::default()`. Enabling it in settings is necessary but not sufficient: the desktop UI's `CloudConfirmDialog` requires an explicit confirmation showing the exact text about to be sent before every individual request, and `translate_natural_language_query`'s `use_cloud` flag is re-checked against stored settings server-side rather than trusted from the caller, so both the UI's gate and the backend's own check have to agree. Only the user's own typed query text is ever sent; file names, paths, and embeddings never leave the local provider path.

The cloud API key is stored in the same local SQLite database as everything else in File Atlas (`ai_settings`, a key-value table), in plaintext. This is a single-user, local-first desktop app with no encryption-at-rest anywhere else in the index either, so this is consistent with the existing trust model, not a new category of risk. OS keychain integration would be a real feature of its own (and, like `atlas-platform`, would need a per-OS implementation) and is not attempted here.

## Decision: "smart suggestions" is the translation UX itself, not a new LLM-generated recommendation type

M5 already ships rule-based, explainable cleanup recommendations with a stated "no black boxes" ethos. An LLM-generated recommendation type (distinct from M5's rules) would introduce a new hallucination/trust surface — "the AI thinks you should delete this" is a materially different claim than "this folder has 90 empty subfolders" — and was not built. "Smart suggestions" for M9 is satisfied by the natural-language search experience itself: showing the user exactly what their request was translated to before running it is the same transparency principle M5 established, applied to search instead of cleanup.

## A real concurrency bug found by clippy, not by testing

The first draft of `get_ai_status` and `semantic_search_files` locked the app's single shared SQLite connection (see ADR 0003's single-writer model), then made a network call to Ollama while still holding that lock, then used the connection again afterward. `cargo clippy`'s `significant_drop_tightening` lint flagged both sites. This was not a style nit: holding the app's one shared connection locked for the duration of an HTTP request (up to a 60-second timeout) would freeze every other database-touching Tauri command — search, home view, everything — for as long as Ollama took to respond, or until it timed out if Ollama had hung. Fixed by splitting `atlas_ai::embeddings::semantic_search` into `rank_by_similarity` (pure SQL, no network) and having callers embed the query first, outside any lock, then take the connection only for the ranking read. `get_ai_status` similarly reads settings, drops the lock, makes its network calls, then re-locks only for the final count query. Recorded here because it is the kind of bug that would not show up in any of this milestone's unit or integration tests (none of them exercise concurrent command calls), only in real usage — exactly the value of running clippy across the actual command layer rather than treating it as a formality.

## Verification

Ollama was already running locally with `nomic-embed-text` installed; `llama3.2:1b` (~1.3 GB) was pulled with explicit user permission specifically so the translation path could be verified against a real small model rather than only a scripted fake, since generating a plausible-looking translation and correctly handling a real model's actual quirks (see above) are different claims. `atlas-ai` ships 19 unit tests against fakes plus 4 tests marked `#[ignore]` that hit the real local Ollama instance (`cargo test -p atlas-ai -- --ignored`), verifying real HTTP calls, real embedding dimensionality (768, matching `nomic-embed-text`), and a real model's translation output surviving validation.

Building the search index is purely additive local metadata (new `file_embeddings` rows keyed by path), not a destructive operation on any real file, so real-app verification against the maintainer's actual 315 GB index is safe the same way M6/M7's read-only verifications were, without needing M4/M5's isolated-database-swap technique. Real-app verification via computer-use confirmed: live Ollama/model status reporting; `build_search_index` showing real progress and `cancel_search_index` stopping it cleanly with the status banner correctly reflecting the partial count afterward (713 of 26,790 files embedded when cancelled); semantic search over that partial index correctly grouping real, unrelated-by-name files from the same real project by meaning (a "video game files" query surfaced sprite images, background art, and a build file all from one real game-project folder, ranked by descending similarity); and the cloud confirmation dialog correctly showing the exact query text and destination model before any send. Two of the three real bugs recorded above (the comma-mangling and the unbalanced quote-stripping) were found during this same verification pass, not by the unit tests written beforehand — a direct instance of why running the actual feature against a real model matters even when the logic already has test coverage.

## Consequences

Positive:

- No new SQL injection surface: the LLM only ever produces a string that has to survive the same parser M3 already hardened.
- The embedding storage schema (path + model + vector) does not need to change when extracted-content embeddings are added later; only what gets embedded changes.
- The connection-locking bug was caught before it ever reached a real multi-command race, by static analysis rather than by a flaky bug report.

Negative / accepted limitations:

- Semantic search only "sees" file names, categories, and folder context, not what is actually inside a file. A file whose content is relevant but whose name is generic will not surface well yet.
- Small local chat models do not always choose the ideal filter primitive; free-text fallback within a translation (not a full fallback to the raw query) is treated as acceptable rather than a defect.
- Cloud provider support is generic (`/chat/completions`-shaped) rather than deeply integrated with any one vendor's specific features (function calling, JSON mode); this is deliberately the smallest useful surface, not a final design.

## Related work

- `crates/atlas-ai/src/{provider,ollama,cloud,embeddings,query_translation,settings}.rs`
- `crates/atlas-db/migrations/0005_ai.sql`
- `apps/desktop/src-tauri/src/ai_commands.rs`
- `apps/desktop/src/components/{AiSearchView,AiSettingsPanel,AiStatusBanner,CloudConfirmDialog,SemanticResultsList}.tsx`
- ADR 0003 (SQLite single-writer model) — the concurrency assumption the lock-tightening fix protects
- ADR 0004 / ADR 0006 (guardrails and safety pipeline) — the safety posture that ruled out LLM-generated SQL
- ADR 0008 (storage map aggregation) — the precedent for linear-scan-over-index instead of a new indexing dependency
