# deino-core

## Role

Deino's features as Tauri-free modules: the rules, the ports they need and
the SQLite adapters behind those ports.

## Owns

- `meetings`: `MeetingService` (list, get, rename) over `MeetingRepo`;
  `MeetingLifecycleService` (status transitions by compare-and-set) over
  `MeetingLifecycleRepo` (create, timing, audio paths, calendar link,
  template); `MeetingTrashRepo` (trash, restore, delete). One
  `SqliteMeetingRepo` and one `FakeMeetingRepo` implement all three ports.
- `settings`: the typed, versioned `Settings` document and its sections;
  `SettingsService` (get, merge-patch, typed update, change events);
  `SettingsPatch` (RFC 7396, validated back into `Settings`);
  `JsonFileSettingsStore` (atomic `settings.json`); and `legacy`, the
  one-time importers of the pre-FND-06 stores (`LegacySqliteImport`, a
  `deino_db::DataMigration`, and `LegacyFileImport`). Fakes:
  `FakeSettingsStore`, `RecordedSettingsEvents`.
  `LlmAccess` implements the LLM gateway's `CredentialSource` (Keychain),
  `EndpointSource` and `RouteStore` (`ai.routes`) ports.
- `summaries`: the legacy template summaries (chunk → combine → fill the
  template) as `SummaryGenerator` over `deino_llm::Gateway`, with the
  reasoning cleanup and chunking they need. FND-08 makes it a pipeline stage;
  NOT-02 replaces it.
- The core-schema aggregates (FND-07), each with small ports, a SQLite
  repository and a fake that pass the same contract tests; feature cards add
  the services and commands:
  - `segments`: `SegmentReader`, `SegmentWriter` (batched, one transaction per
    batch) and `SpeakerRepo`;
  - `people`: `PersonRepo` and `AttendeeRepo`;
  - `companies`: `CompanyRepo`;
  - `calendar`: `CalendarAccountRepo` and `CalendarEventRepo` (upsert by
    provider ID);
  - `folders`: `FolderRepo` (the only writer of `parent_id`, refusing
    cycles) and `MeetingFolderRepo`;
  - `tags`: `TagRepo` and `MeetingTagRepo`;
  - `action_items`: `ActionItemRepo`.
- `_example` (debug builds only): the reference feature to copy, with its own
  in-memory schema in `src/_example/migrations/`.

Each feature is `<feature>.rs` plus `<feature>/`:

| File | Layer |
|---|---|
| `model.rs` | Types, validated on construction |
| `error.rs` | A `thiserror` enum implementing `ErrorClass` |
| `repo.rs` | The storage port and its SQLite adapter (`sqlx::query*!`) |
| `service.rs` | Rules over ports and `Clock`; no SQL, no Tauri |
| `fake.rs` | In-memory ports behind `test-support` |
| `queries/*.sql` | Queries a test inspects with `EXPLAIN QUERY PLAN` (`testing::query_plan`) |
| `*_tests.rs` | Contract tests run against both the SQLite repository and the fake |

## Does Not Own

- IPC, events and wiring: the app crate (`features/`, `composition.rs`).
- Opening the database and migrations: `deino-db`.
- IDs and the error model: `deino-domain`.

## Invariants

- No dependency on `tauri`. Ever.
- Services depend on traits (`Arc<dyn Port>`), never on concrete adapters.
- Each port method is one unit of work; multi-statement writes run in one
  transaction on the writer.
- Time comes from `Clock`; logs carry IDs and counts, never content.
- Settings never hold a secret; keys go through `deino-secrets`. A settings
  file from an older build always loads (every section is `#[serde(default)]`).
- Fakes honour their port's whole contract, ordering included.
