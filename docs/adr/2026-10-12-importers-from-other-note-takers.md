# Importers from other note-takers: one adapter per source, one import path, a source-ID ledger

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** CAP-12 (importers); builds on DEV-03's `TranscriptImporter`
  (ADR 2026-10-12-local-rest-api), ADR 0019 (network egress), ADR 0031
  (credentials in the Keychain), ADR 0036 (entities) and CAP-11's ledger
  (ADR 0044)

## Context

CAP-12 brings meetings over from Granola, Circleback, Fireflies and Fathom
(a pasted API key each) and from a folder of exported files: notes, the
transcript with speakers and times, attendees and action items. Settings →
Import shows a count first, then runs in the background with progress and
can be stopped. Importing twice must add nothing, and imported meetings must
be searchable and reachable from the Ask panel.

Questions: where provider code lives; how a meeting is written; what
"already imported" means; where keys and requests go; how the job runs.

## Decision

1. **Rules in `deino-core::importers`, providers in a new crate,
   `deino-importers`.** The core owns the `Importer` port (`page(cursor)`
   and `fetch(item)`), the `ImportRunner` (preview, run), the `ImportJobs`
   slot (one job at a time, a `CancellationToken`, the last progress) and
   the ledger. Each provider is one module implementing `Importer`
   (Open/Closed); adding one is a module plus one arm in
   `ImportSources::provider`. Provider wire types never reach the core,
   like `deino-connect` for calendars.
2. **One import path.** Every adapter maps its meeting into DEV-03's
   `TranscriptImport` (Circleback's import shape), which gains `attendees`,
   `actionItems` and an optional `endTimestamp` per utterance, and is
   written by `TranscriptImporter`. `TranscriptImporter::with_people` adds
   attendees through ORG-04's `EntityDirectory` (keyed by email, else name;
   companies from work domains) and action items through AI-05's
   `ActionItemRepo` as `ai`-made items (no INT-03/AUT-01 events fire for
   history). The importer schedules the `index` stage; keyword search sees
   the rows at once through the search triggers, so the Ask panel's
   retrieval finds them too.
3. **The ledger is `imported_meetings (source, source_id) → meeting_id`**,
   not a column on `meetings`: it keeps working after a meeting is deleted
   (`ON DELETE SET NULL`, so a deleted meeting never comes back, like
   CAP-11's `watched_files`), and `meetings` is not rebuilt. A folder file's
   ID is the SHA-256 of its bytes. A meeting that fails is not recorded, so
   the next run tries it again.
4. **Keys in the Keychain, requests through `deino-net`.** Four
   `SecretName`s (`granola_api_key`, ...) and four `ServiceId`s with one
   host each (`public-api.granola.ai`, `circleback.ai`, `api.fireflies.ai`,
   `api.fathom.ai`). Saving a key switches the service on (as connecting an
   integration does), forgetting it switches it off; offline mode blocks
   imports and the egress log records every request. 401/403 end the run
   as a refused key; 429 waits out `Retry-After` (capped at 30 s, three
   tries).
5. **The job runs on the app's runtime.** `start` claims the slot, spawns
   `ImportJobs::run` and returns; progress is a typed event and `status`
   answers after a reload. Stop takes effect between meetings, so no
   meeting is half-written by a cancel.

## Consequences

- Tests replay recorded answers written from each provider's public API
  documentation (`crates/deino-importers/fixtures/`); no test reaches a
  real API. The fixtures are only as accurate as those documents.
- Fireflies and Fathom list whole meetings, so a preview downloads them
  too; Granola and Circleback list summaries and fetch each meeting.
- Granola's 413 `TRANSCRIPT_TOO_LARGE` imports the note without its
  transcript (paging `/transcript` is a follow-up).
- Imported notes are stored as AI notes version 1 (`model` is DEV-03's
  `deino:api-import`), like the API's imports.
