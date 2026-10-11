# deino-db

## Role

Deino's SQLite substrate: opening the database, its schema history, and
commit-time change signals (ADR 0005).

## Owns

- `Database` (`database.rs`): WAL, foreign keys on, a 5 s busy timeout, one
  writer connection and a `query_only` read pool; `close()` checkpoints the WAL.
- `MIGRATOR` and `migrations/`: the one schema history for `deino.sqlite`.
- `DataMigration` (`data_migration.rs`) and `Database::open_with`: Rust steps
  that run against the schema just before a given migration, for data that
  must leave the database before that migration (FND-06 moves API keys to
  the Keychain before dropping `settings`).
- `ChangeNotifier` (`change_notifier.rs`): update/commit/rollback hooks on the
  writer that publish `TableChanges { tables, seq }` once per committed
  transaction. Adapted from anarlog's `db-change` (MIT).
- `Table` (`table.rs`): every table the UI can be told about.
- The core schema (FND-07, ADR 0012), including the legacy `transcripts`
  compatibility view over `transcript_segments`, and the pipeline queue
  tables `jobs` and `job_runs` (FND-08, ADR 0018; their SQL lives in
  `deino-pipeline`), and `templates` / `template_sections` with the
  `meetings.template_id` reference (NOT-04, ADR 0021; SQL in
  `deino-core::templates`).
- `Database::open_read_only` (`read_only.rs`, DEV-02/04, ADR 0035): the
  existing file opened `mode=ro`, `query_only`, in WAL mode with the same
  busy timeout, for the `deino` CLI; never creates, migrates or writes it, and
  refuses a schema whose applied migrations differ from the binary's
  (`SchemaCheck`: newer or older).
- `test_db()` (feature `test-support`): an in-memory database with every
  migration applied.

## Does Not Own

- Feature SQL: each feature's `repo.rs` in `deino-core`.
- Turning changes into IPC events: the app's `features/db.rs`.
- Legacy corruption recovery (deleting WAL side files): the app's
  `database/manager.rs`.

## Invariants

- A `DataMigration` runs only while its migration is unapplied, and must be
  idempotent: a crash between the step and the migration runs it again.
- Migrations are append-only; never edit an applied file. Name new ones
  `<UTC YYYYMMDDHHMMSS>_<card>_<desc>.sql`, then run `scripts/sqlx-prepare.sh`.
- Every write goes through `writer()`, so it passes the change hooks.
- A new table gets a `Table` variant in the same PR.
- Hook callbacks stay cheap: no allocation for untracked tables, no I/O, no
  async work. Signals are emitted on commit and dropped on rollback.
- The update hook cannot see `DELETE` without `WHERE` or `REPLACE` conflicts,
  nor `WITHOUT ROWID` tables: never create one, and write upserts as
  `ON CONFLICT DO UPDATE`.
- A view never reaches the update hook; writes through `transcripts` are
  reported as `transcript_segments`.
