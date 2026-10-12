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
  The Meetings page (ORG-07, ADR 0046): `MeetingListService` (filtered
  cursor pages, `matching_ids`), `MeetingViewService` (saved views),
  `TrashService` and `TrashPurger` (batch trash and restore, delete
  forever with the files, the 30-day purge), `MeetingMergeService`,
  `MeetingBulkService`, and the `MeetingWork` port through which trashing
  cancels a meeting's pipeline and a merge re-indexes.
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
    batch), `SegmentEditor` (TRN-02: one segment in any state, batched text
    edits, the find and replace pre-filter) and `SpeakerRepo`;
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
    `AttendeeDirectory` (attendees' people and companies, through
    `entities::EntityDirectory`);
    `CalendarService` and `SyncScheduler`. No network client: web
    providers live in `deino-connect`. `calendar::linking` (ORG-03, ADR
    0034): the nearest-event rule detection shares, `EventLinkService`
    (auto-link, link, re-link, unlink), `EventAttendance` (invitees into
    attendees) and `RelatedService` (series, or title and people);
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
  - `insights` (AI-06, ADR 0038): `InsightRepo`, `InsightResultRepo`,
    `SpeakerNames`, `MeetingRecords` and `InsightExports`, all on
    `SqliteInsightRepo` (`insight_defs`, `insight_results`); `InsightService`
    (definitions with revisions, the 12-template `gallery`, a meeting's tab,
    an insight's table, run candidates, runs on past meetings through the
    `RunScheduler` port, CSV through `deino_export::csv::insight_table`) and
    `GenerateService` (the `insights` stage's rules: requested runs, else
    matching automatic insights; `InsightModel` with `GatewayInsightModel`
    on route `extract.insights` and a per-insight JSON schema; `validate`
    types every field, repairing close values; `Attribution` resolves `s12`
    references to speaker, time and segment). Fakes: `FakeInsightStore`,
    `ScriptedInsightModel`, `RecordedScheduler`, `FixedDomains`.
  - `automations` (AUT-01, ADR 0042): `AutomationRepo` and `RunRepo` on
    `SqliteAutomationRepo` (`automations`, `automation_runs`,
    `automation_run_steps`); the `Step` trait and `StepRegistry` (one step per
    `ActionKind`, the picker's `catalog` with provider kinds as WIP); the
    local steps (`steps/`: insight, tag, folder, export, notification, shell
    script behind the `ShellSwitch`, 60 s, killed as a process group);
    `AutomationPayload`; `AutomationEngine` (the `automations` stage's
    rules: idempotent per automation and meeting, resumes after done steps,
    stop on failure, retryable vs stored failures); `AutomationService`
    (definitions, duplicate, Run on selected, Test on latest, the tag
    trigger, the meeting menu, matching meetings, the run log). Fakes:
    `ScriptedStep`, `RecordedAutomationScheduler`.
- `stats` (ORG-09, ADR 0049): `StatsRepo` / `SqliteStatsRepo` (four reads over
  the `stats_meetings` view and `llm_calls`), `StatsService` (a period's
  numbers and the last year's heatmap, aggregated on read in the user's
  `LocalZone`; `pricing.rs` estimates cost from list prices) and
  `RecapService` (the year in review on route `recap`, from numbers and
  titles only, kept once per year in `year_recaps` through `RecapRepo`).
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
  `Backfill` and `SemanticAdmin`. `ReindexQueue` (TRN-02) re-indexes
  meetings after transcript edits, one at a time on a tracked task. Fakes:
  `FakeEmbedder`, `FakeSemanticSearch`.
- `transcript` (TRN-01, ADR 0033): `TranscriptService::transcript`, one
  meeting's whole `Transcript` (segments in spoken order, each with its
  `speaker_id`) over `SegmentReader`; the view names speakers from
  `speakers` (CAP-04). No SQL of its own. TRN-02 (ADR 0040):
  `TranscriptEditService` edits a line (`original_text` keeps the engine's
  text), soft-deletes and restores one, and finds and replaces across one
  meeting or all (a `ReplacePlan` that is both the preview and the edit),
  then queues the meetings on its `TranscriptIndexer` port.
- `player` (TRN-03, ADR 0040): `PlayerService` lists a meeting's playable
  recording files inside the recordings folder (`MediaFiles`,
  `FsMediaFiles`) and downloads one through the export's `SaveDialog`.
- `query` (DEV-02/04, ADR 0035): the read side the `deino` CLI and its MCP
  server answer from. `Library` bundles `MeetingQueries` (list by date, tag,
  folder, person and company; read one with its notes, the newest enhanced
  notes and their cited moments, action items and files),
  `TranscriptQueries` (time windows with a cursor), `ActionItemQueries`
  (over `ActionItemService`), `DirectoryQueries` (people, companies, tags,
  folders, calendar events) and the `Search` port (`HybridSearch` over
  `search::Retriever`, reporting keyword or hybrid; `EmbedProbe`), all over
  `ReadPorts` and `ReadContext`, the app's own
  repositories and services. `Resolver` turns names, emails and domains into
  IDs; `link` writes `deino://meeting/<id>[?t=<ms>]`. `seed_sample_library`
  (feature `test-support`) seeds the fixtures plus tags, folders, action
  items and AI notes for tests and demos.
- `cli_setup` (DEV-02/04): `CliSetupService` (where the bundled CLI is, its
  install targets `~/.local/bin` and `/usr/local/bin`, installing it as a
  symlink that never replaces a file it did not make), the `Links` port
  (`FsLinks`) and the Claude Code and Claude Desktop `snippets`.
- `entities` (ORG-04, ADR 0036): `EntityDirectory`, the one set of upsert
  rules for people (email, else exact name) and companies (email domain,
  never a public or personal one), following `merged_into`;
  `PersonService` and `CompanyService` (lists with meeting counts and last
  met computed on read, pages, add, edit, notes, merges); `EntityMerger`
  (`SqliteEntityRepo`: one transaction that moves attendance, speakers,
  voiceprints, suggestions, action items and @mentions, and flattens
  earlier merges); `AttendanceService`; `EntitySummarizer` (route
  `entity.summary`); `LogoService` over the `LogoSource` (`FaviconSource`,
  `deino-net`'s `favicons` service) and `LogoFiles` (`FsLogoFiles`) ports;
  `EntityUpdateService`, the `update_entities` stage's port. A table that
  references `people` adds its move to `merge_repo.rs`.
- `vocabulary` (TRN-05, ADR 0037): `VocabularyService` (entries in
  `vocab_terms` and `vocab_variants`: terms with sounds-like spellings and
  replacement rules), the pure `Lexicon`, matcher and `Corrector` (whole
  words, ignoring case; Jaro-Winkler for misheard names), `NameSources`
  (people, companies, a meeting's invitees), `CorrectionRepo`
  (`vocab_corrections`: originals for undo), `LiveVocabulary` (the
  recorder's live pass), `FixupService` (the `vocab_fixups` stage's rules)
  and `PromptVocabulary` (the enhanced notes' terms). Fakes:
  `RecordedLiveVocabulary`, `NoLiveVocabulary`.
- `final_transcription` (CAP-08, ADR 0037): `FinalTranscriptionService`
  (the `final_transcription` stage's rules: transcribe a record-only
  meeting's tracks, or re-transcribe with the high-accuracy re-pass) over
  the `TrackTranscriber` port, and `carried_speaker`. Fake: `ScriptedTracks`.
- `redaction` (NOT-09, ADR 0043): pure detectors, one per `SecretKind`, over
  typed and spoken text; `Redactor` with the optional `SpanCheck`
  (`LocalModelCheck`: the built-in model only, may only reject); the
  `TextRedactor` port used by `MaterialSources` and `ExportPorts`, and
  `SettingRedactor` over `privacy.redactSensitive`. No storage: secrets
  are redacted on the way out; the transcript keeps them.
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
- A transcript line the user deleted (`is_deleted`) is read by nobody: every
  reader goes through `SegmentReader::list` or SQL that skips it, and its
  semantic windows leave `embeddings` in the deleting transaction.
- An automation runs at most once per meeting on its own; only a manual run
  replaces a run. A shell script runs only while `automations.shellScripts`
  is on, read at every run.
