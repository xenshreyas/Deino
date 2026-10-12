# deino-importers

## Role

Imports meetings from other note-takers (CAP-12): Granola, Circleback,
Fireflies and Fathom with an API key the user pastes, and folders of JSON or
Markdown files. Each source is one adapter behind
`deino_core::importers::Importer`. The crate exists so that `deino-core`
holds the rules (the runner, the de-duplication ledger, the job) and no
provider's wire format.

## Owns

- One module per source (`granola`, `circleback`, `fireflies`, `fathom`,
  `folder`): its wire types, its paging and the mapping of notes,
  transcript (speakers and times), attendees and action items into a
  `deino_core::transcript_import::TranscriptImport`.
- `http.rs`: `ProviderApi`, the key header, 401/403 as a refused key,
  waiting out 429s (`Retry-After`, capped), RFC 8288 `Link` paging.
- `ImportSources`: keys in the Keychain (`SecretName::*ApiKey`), the
  provider's `ServiceId` switched on when a key is saved, and the adapter
  for an import.
- `fixtures/`: API responses written from each provider's public API
  documentation, which the tests replay. Tests never call a real API.

## Does Not Own

- The runner, the preview count, the ledger (`imported_meetings`), the job
  and its progress: `deino_core::importers`.
- Writing a meeting (speakers, segments, notes, tags, attendees through
  ORG-04's `EntityDirectory`, action items): DEV-03's
  `deino_core::transcript_import::TranscriptImporter`, the one import path.
- The HTTP client, host lists, switches, offline mode and the egress log:
  `deino-net` (`ServiceId::{Granola, Circleback, Fireflies, Fathom}`).

## Invariants

- Every request goes through a `ScopedClient` for the source's service.
- A key never appears in settings, logs or errors.
- Nothing logs meeting titles, notes or transcript text.
- Utterances reach the importer in time order; blank lines are dropped.
