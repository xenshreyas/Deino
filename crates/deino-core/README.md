# deino-core

## Role

Deino's features as Tauri-free modules: the rules, the ports they need and
the SQLite adapters behind those ports.

## Owns

- `meetings`: `MeetingService` (list, get, rename) over `MeetingRepo`;
  `MeetingLifecycleService` (status transitions by compare-and-set) over
  `MeetingLifecycleRepo` (create, timing, audio paths, calendar link,
  template), which announces every stored change to subscribed
  `MeetingStatusObserver`s (the pipeline's trigger, FND-08);
  `MeetingTrashRepo` (trash, restore, delete). One
  `SqliteMeetingRepo` and one `FakeMeetingRepo` implement all three ports.
- `settings`: the typed, versioned `Settings` document and its sections
  (including `network`: offline mode and the per-service switches that
  `deino-net`'s policy reads, FND-12);
  `SettingsService` (get, merge-patch, typed update, change events);
  `SettingsPatch` (RFC 7396, validated back into `Settings`);
  `JsonFileSettingsStore` (atomic `settings.json`); and `legacy`, the
  one-time importers of the pre-FND-06 stores (`LegacySqliteImport`, a
  `deino_db::DataMigration`, and `LegacyFileImport`). Fakes:
  `FakeSettingsStore`, `RecordedSettingsEvents`.
  `LlmAccess` implements the LLM gateway's `CredentialSource` (Keychain),
  `EndpointSource` and `RouteStore` (`ai.routes`) ports.
- `enhance` (NOT-02, ADR 0030): the enhanced notes. `EnhanceService`
  gathers a meeting's material (`MaterialSources`: transcript as short
  references, the user's notes, the calendar event, attendees, the series'
  previous notes, vocabulary), asks `notes.enhance` for a structured
  `EnhancedDoc` (`NotesWriter`: one pass, or ~20-minute parts and a merge),
  then `assemble` (the provenance guard and `SegmentIndex`) and `to_doc`
  (origin marks, citations). Versions live in `enhanced_notes`
  (`SqliteEnhancedNotesRepo`: `EnhancedNotesReader`, `EnhancedNotesWriter`);
  the newest is editable, an edited one is never replaced without
  `Overwrite::ReplaceEdits`. `EnhanceBasis` yields the regenerate reasons,
  `AutoTitler` names placeholder-titled meetings, `LegacySummaryImport`
  copies the old summaries in as version 1. Fake: `FakeEnhancedNotesRepo`.
- The core-schema aggregates (FND-07), each with small ports, a SQLite
  repository and a fake that pass the same contract tests; feature cards add
  the services and commands:
  - `diarization` (CAP-03): `DiarizeService` (the `diarize` stage's rules)
  over the `TrackDiarizer` engine port, `plan_channels`, `attribute` and
  `known_voices` (pure), `FakeTrackDiarizer`. The engine lives in
  `deino-diarize`. `diarization::identify` (CAP-04, ADR 0029): the
  `identify_speakers` stage's rules: `Lineup` (reads the meeting), the pure
  `decide` (by voice with margin and mutual best, by elimination,
  suggestions) and `IdentifyService` over `IdentifyPolicy`
  (`Settings::speakers`) and `IdentificationWriter`.
  - `voiceprints` (CAP-04): `VoiceprintRepo` and `ExemplarReader` over the
  `voiceprints` table (five best exemplars per person, evicted in the
  enrolment's transaction), `VoiceprintService` (enrolment from a confirmed
  rename only, quality from clean spans, profiles, delete, forget), the
  pure `matching` primitives and enrolment `spans`; `FakeVoiceprintRepo`.
  - `speakers` (CAP-04): `ParticipantReader`, `NamingRepo` and
  `IdentificationWriter` (`SqliteNamingRepo`, `speaker_suggestions`),
  `SpeakerService` (participants, candidates, rename one segment or all,
  merge, suggestions; action items follow the voice).
- `segments`: `SegmentReader`, `SegmentWriter` (batched, one transaction per
    batch) and `SpeakerRepo`;
  - `people`: `PersonRepo` (with `search` by name, word or email prefix),
    `AttendeeRepo` and `MeRepo` (the one person who is the user, moved in one
    transaction); `PeopleService::search` feeds the notepad's @mentions;
  - `companies`: `CompanyRepo`;
  - `calendar` (ORG-01, ADR 0027): `CalendarAccountRepo`,
    `CalendarEventRepo` (upsert by provider ID, change-only) and
    `CalendarRepo`; the `CalendarSource` port that `deino-eventkit` and
    `deino-connect` implement; the conference-link parser, the filters, the
    mapping into a `SyncBatch` and `CalendarSyncRepo::apply` (one
    transaction, per-provider pruning, `record_override` never written);
    `AttendeeDirectory` (people and companies from attendees);
    `CalendarService` and `SyncScheduler`. No network client: web
    providers live in `deino-connect`;
  - `folders` (ORG-05, ADR 0024): `FolderRepo` (the only writer of
    `parent_id`, refusing cycles), `MeetingFolderRepo`, `FolderRuleRepo`
    (`folder_rules`) and `MeetingFactsReader`; `FolderService` (the tree),
    `FilingService` (filings, Unfiled, the series offer, rules), the typed
    `FolderRule` with the pure `folders_matching`, and `AutoFileService`
    (the `auto_folder` stage's rules);
  - `tags` (ORG-06): `TagRepo`, `MeetingTagRepo` and `MeetingDigestReader`;
    `TagService` (CRUD, apply/remove, `MeetingTagged` through `TagEvents`)
    and `AutoTagService` (the `auto_tag` stage's rules: existing tags only,
    on route `tagging`, skipped when the user tagged the meeting);
  - `action_items` (AI-05, ADR 0028): `ActionItemRepo`, `ActionItemBatch`
    (bulk edits and the regeneration that replaces only untouched AI
    items), `ActionItemReader` (three indexed list queries chosen by
    `ItemQuery::access_path`) and `SavedViewRepo` (`action_item_views`);
    `ActionItemService` (list by `ActionItemFilter`, the "Today" digest,
    create, edit, bulk edit, delete, "mark all mine done"; changes through
    `ActionItemEvents`), `SavedViewService`, `DailyReminder` (09:00 local
    over the `Notifier` and `ReminderSwitch` ports), and `extraction`: the
    `extract_action_items` stage's rules (`ContextSources`, the
    `ActionItemModel` port with `GatewayActionItemModel` on route
    `extract.action_items`, and the pure `resolve_due`, `Roster` and
    `Transcript` that turn the model's phrases, names and citations into
    days, people and segments). Fakes: `FakeActionItemExtractor`.
- `models` (CAP-01, AI-01): the bundled model catalog (`catalog.json`: Parakeet
  Unified EN 0.6B, six quants, and Nomic Embed Text v1.5 Q8_0, each pinned to a
  revision, size and SHA-256, each with a purpose; a `ModelService` serves one
  purpose),
  `ModelService` (list with state, recommended quant by RAM, download, cancel,
  delete, ready path), `ModelStore` (`<app_data_dir>/models/parakeet-unified/`)
  and the resumable, verified download over the `ByteSource` port.
  `FakeByteSource` and `RecordedModelEvents` are behind `test-support`. It
  never touches `deino-stt`: the engine only receives a verified path.
- `templates` (NOT-04, ADR 0021): `Template` / `TemplateContent` (sections,
  `SectionFormat`, typed `AutoRule`s), validated on the way in; the
  30-template `CATALOG` and `BuiltinSync` (versioned seeding that never
  touches user templates); `TemplateService` (library, duplicate,
  import/export, the legacy-folder import); `MeetingTemplateService` (a meeting's template and its
  `ChoiceReason`, choosing one, previews) over the pure `RuleEngine` and the
  `TemplateClassifier` port (`GatewayTemplateClassifier` on route `tagging`,
  `NoClassifier`); `SqliteTemplateRepo` (`TemplateRepo`, `BuiltinStore`,
  `MeetingFactsReader`). Fakes: `FakeTemplateRepo`, `FakeMeetingFacts`,
  `FixedClassifier`.
- `profile` (NOT-05): `ProfileService` (normalises a `ProfileInput` into
  `settings.profile`, keeps the "Me" person in step), `render_profile_context`
  and `ProfileContext` (the gateway's `SystemContext`), `InternalDomains`
  (internal vs external by email domain, personal mail providers excluded).
- `notes` (NOT-01): `NoteService` over `NotesRepo` (`meeting_notes`:
  the TipTap document as `deino_doc::Doc` plus its Readable Markdown),
  `MeetingLifecycleRepo` (scheduled meetings for notes written ahead),
  `NoteImages` (`FsNoteImages`: `<root>/<meeting id>/images/<uuid>.<ext>`,
  content-sniffed PNG/JPEG/GIF/WebP up to 10 MB) and `NotesEvents`.
  `adopt` moves the record screen's draft into the meeting a recording
  saved. Fakes: `FakeNotesRepo`, `MemoryNoteImages`, `RecordedNotesEvents`.
- `export` (EXP-01, EXP-02, ADR 0025): `ExportAssembler` (a meeting into
  `deino_export::MeetingExport` over the repositories, the `NoteRenderer`
  port with `DocNoteRenderer`, and the `LocalZone` port), `ExportService`
  (preview, save, copy, email), `AutoExportService` (the `auto_export`
  stage's `AutoExporter`: vault notes by the settings' templates,
  overwriting only its own files, Obsidian people pages, transcript
  sidecars), `FullExportService` (the account export as a tracked task
  writing a zip with CSV tables), the OS ports (`SaveDialog`,
  `FolderDialog`, `Clipboard`, `UrlOpener`) and `Vault` (`FsVault`, atomic
  writes). Fakes: `FakeVault`, `ScriptedDialog`, `RecordedClipboard`,
  `RecordedOpener`.
- `search` (ORG-08, AI-01, ADR 0032): the query language (`query`), FTS5
  expressions (`fts`), snippets with UTF-16 highlight ranges (`snippet`),
  `SqliteSearchIndex` (`SearchIndex`: keyword hits, filter scopes, refresh,
  reindex; `FilterDirectory`: `@` names and known domains), reciprocal rank
  fusion (`hybrid`), `sections`, `SearchService` (`search`, `search_hybrid`,
  `suggest_filters`, `reindex_all`) and `MeetingIndexer` (`SearchIndexing`,
  the `index` stage's port). `Retriever`: read-only hybrid search for
  another process (the `deino` CLI and MCP server) over a read-only pool and
  an optional `Embedder`, keyword-only without one; writes through its
  adapters fail with `SearchError::ReadOnly`. `semantic`: the chunker, the `Embedder` port
  (`GatewayEmbedder`, nomic task prefixes; `NoEmbedder`), `SqliteEmbeddingStore`
  (`MeetingSourceReader`, `EmbeddingStore`), `VectorIndex` behind
  `SharedVectors`, `SemanticIndexer`, `SemanticIndex` (`SemanticSearch`), the
  `Backfill` and `SemanticAdmin`. Fakes: `FakeEmbedder`, `FakeSemanticSearch`.
- `transcript` (TRN-01): `TranscriptService::transcript`, one meeting's
  `Transcript` (segments in spoken order and `TranscriptSpeaker`s with the
  name the view shows: the identified person's, following a merge, else the
  label) over `SegmentReader`, `SpeakerRepo` and `PersonRepo`. No SQL of its
  own; TRN-02 adds the segment edits here.
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
- Built-in templates are written only by `BuiltinSync`; user templates
  (`builtin_version IS NULL`) only by `TemplateService`. At most one person
  is the user.
- Settings never hold a secret; keys go through `deino-secrets`. A settings
  file from an older build always loads (every section is `#[serde(default)]`).
- Fakes honour their port's whole contract, ordering included.
