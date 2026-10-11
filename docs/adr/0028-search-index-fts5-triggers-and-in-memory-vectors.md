# 0028. Search: an FTS5 index kept by triggers, and an in-memory vector index fused by RRF

- **Status:** Accepted
- **Date:** 2026-10-11
- **Card:** ORG-08 (Global search and ⌘K) and AI-01 (Embeddings and semantic index)

## Context

Deino needs one retrieval layer (PARITY_PLAN §4.4) for the ⌘K palette, the
results page and, next, chat (AI-02), MCP and the API: keyword search over
meeting titles, the user's notes, summaries, transcript moments, action items,
people, companies and folders, with `@` filters and attendee domains
(Circleback §9.1), plus search by meaning with a local embedding model
(`nomic-embed-text-v1.5`, always on the Mac, §8 D3).

The index must be correct after every write, including the legacy code that
still writes `transcripts` through its view (ADR 0012), soft-deleted
segments and trashed meetings. It must answer in under 100 ms over 2,000
meetings, and the semantic part must find a paraphrased question's meeting in
the top three for 8 of 10 test questions.

## Decision

**Keyword: FTS5 over a derived documents table, kept by triggers.**

- `search_documents` (STRICT) holds one row per searchable thing: `kind`
  (`meeting`, `note`, `summary`, `segment`, `action_item`, `person`,
  `company`, `folder`), `ref_id`, `meeting_id` (FK, cascade), `title`, `body`,
  `ts_ms`, unique on `(kind, ref_id)`. `search_fts` is its FTS5 index as an
  external-content table (the columns §4.4 names, the bookkeeping ones
  `UNINDEXED`), so FTS5 gets a stable integer rowid and triggers find a
  document through a B-tree index, never a full-text scan.
- **Triggers, not a stage.** Each source table has insert/update/delete
  triggers that "refresh" its document in the same transaction as the
  write; three triggers on `search_documents` mirror it into `search_fts`.
  Every writer is covered, legacy ones included, and there is no window in
  which a fresh transcript line or a renamed meeting is missing. Soft-deleted
  segments, emptied notes and merged people leave the index; trashed meetings
  stay indexed and are filtered at query time (restoring needs no reindex).
  The `index` pipeline stage still re-derives a meeting's documents
  (`refresh_meeting`, cheap and idempotent) and `reindex_all` rebuilds
  everything and runs FTS5 `rebuild` + `optimize` to repair any drift; both
  use the same SQL rules as the triggers (`queries/documents/*.sql`).
- **Tokenizer:** `porter unicode61 remove_diacritics 2` with `prefix = '2 3'`.
  Stemming matches "shipping" to "ship"; the prefix indexes serve the
  palette's type-ahead (`"roadm"*` on the last word while typing).
  **No trigram table:** substring matches inside words rarely matter for
  meeting search, a trigram index would roughly double the index, and it
  cannot stem or rank with BM25 the way the porter table does.
- **Ranking:** BM25 with a title weighing four times a body
  (`bm25(…, 4.0, 1.0)`), set as the table's default `rank` so unfiltered
  searches take FTS5's `ORDER BY rank LIMIT 1000` path before the per-kind
  cut; `highlight()`/`snippet()` (marked with STX/ETX, turned into UTF-16
  highlight ranges for JavaScript) run only for the rows that survive.
- **Query language** (`search::query`): words (AND; the last one a prefix
  while typing), quoted phrases, `@mentions` (optionally `@kind:`), and bare
  domains. Mentions resolve to people, companies, tags, folders or meetings
  by exact name, else by a unique name prefix; domains only filter when some
  attendee or company uses them (so `next.js` stays a word). Filters AND
  together and are computed once as a meeting scope (`scope.sql`, JSON arrays
  through `json_each`, folders with their subfolders), which both the
  keyword and the semantic search use.

**Semantic: chunks embedded by llama-helper, an exact in-memory cosine index.**

- **Model:** `nomic-embed-text-v1.5` Q8_0 GGUF (146,146,432 bytes, SHA-256
  `3e243421…c3b7`, revision `0188c9b`, Apache-2.0) in the model catalog with
  a purpose (`embedding`), downloaded by the same verified `ModelService`
  through the approved model-download client into `<models>/embedding/`.
  It runs through the gateway's `embed` (route `Embed`, always local) in
  llama-helper, with the model's `search_document:` / `search_query:`
  prefixes. llama-helper's `embed` turned out suitable (mean pooling,
  L2-normalised, 768 dimensions), so no ONNX path was needed.
- **Chunks:** transcript windows of about 400 tokens cut at speaker turns
  (`Speaker: text` lines), each window starting with the previous window's
  last turn when it is short (≤ 100 tokens); notes and summary sections
  (split between paragraphs when long). Stored in `embeddings` with
  `ref_kind`, `ref_id`, `meeting_id`, `chunk_ix`, `ts_ms`/`end_ms`, the text
  and the model; re-indexing reuses the vector of every chunk whose text and
  model are unchanged.
- **Vector store: BLOBs plus an in-memory index, not sqlite-vec.** We tried
  `sqlite-vec` 0.1.6 (MIT/Apache-2.0): it links statically and loads through
  `sqlite3_auto_extension`, and its `vec0` brute-force KNN over 40,000 × 768
  vectors took ~26 ms. We did not adopt it because loading it needs `unsafe`
  FFI and a process-global registration (the workspace denies
  `unsafe_code`), `sqlx-cli` cannot load the extension, so a migration
  creating a `vec0` table breaks `scripts/sqlx-prepare.sh` and every
  compile-time-checked query, and its on-disk format is not stable before
  1.0. Instead vectors are little-endian `f32` BLOBs, and `VectorIndex`
  keeps them unit-length in one contiguous matrix, loaded lazily on the
  first semantic search and updated a meeting at a time by the indexer.
  The same 40,000 × 768 search takes ~15 ms and ~120 MB; at personal scale
  that is fine, and exact.
- **When it runs:** the `index` stage (order 80) embeds each processed
  meeting; a backfill (one tracked task, Settings → Search index) embeds
  existing ones and starts by itself after the model downloads. Without the
  model the stage skips with a reason and search stays keyword-only.
- **Query embedding is bounded:** a search waits at most 300 ms for its
  query vector (cached; the sidecar may be busy with a summary), then
  answers by keywords alone. The sidecar adapter reads every embed reply on
  its own task, so a caller that stops waiting never desynchronises the
  helper's line protocol.

**Fusion: reciprocal rank fusion.** `hybrid::fuse` adds `1/(60 + rank)` per
list, ranks counted within each kind. A keyword moment inside a semantic
transcript window takes the window's key, so the two fuse and the result
keeps the keyword hit's exact time and highlights. `SearchService::search`
groups hits for the UI (one row per meeting with its best hit, moments,
people, companies, action items, folders); `SearchService::search_hybrid`
is AI-02's retrieval API: fused hits (passages with meeting, time window and
text), matching a question's words with OR.

## Alternatives considered

- **A pipeline stage plus `DbChanged` hooks to keep FTS in step:** leaves
  windows where the index lags (live recording, legacy writes outside the
  pipeline) and needs per-table refresh logic in Rust; triggers are atomic.
- **A plain (non-external-content) FTS5 table:** deleting a document by
  `(kind, ref_id)` would scan the full-text table on every segment edit.
- **sqlite-vec:** see above. Revisit when it reaches 1.0 and sqlx can load
  extensions at prepare time, or when a user's index outgrows memory.
- **ONNX embeddings through `ort`:** unnecessary while llama-helper serves
  the GGUF; it would add a second runtime path for one model.

## Consequences

- Every write to an indexed table also writes `search_documents` (and FTS5's
  shadow tables) in the same transaction, so `DbChanged` reports
  `search_documents` alongside the source table; search queries listen to it.
- A new searchable source is a trigger set plus a `queries/documents/` file
  and a `HitKind`; NOT-02's `enhanced_notes` replaces the legacy summary
  triggers (follow-up).
- Measured: 2,000 synthetic meetings (100,000 segments, a 24-word vocabulary
  so common words hit half of all segments) answer in 39–62 ms (slowest of
  three runs per query) at load ~40 in a release build (`search::bench_tests`); the semantic eval finds 10 of
  10 paraphrased fixture questions' meetings in the top three with the real
  model (`llm::retrieval_eval`).
- Memory grows by about 3 KB per chunk while the semantic index is loaded.
