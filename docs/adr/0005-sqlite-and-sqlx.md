# 0005. Store data in SQLite through sqlx, owned by `deino-db`

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13

## Context

All user data lives on the Mac: meetings, transcripts, notes, later people,
action items, embeddings and jobs. The inherited code used sqlx 0.8 with
runtime-checked queries, migrations in the app crate, one pool for everything
and no change notifications. PARITY_PLAN §3A asks for compile-checked queries,
WAL with one writer and a read pool, foreign keys on, indexes asserted by
tests, and multi-row writes in transactions. `docs/references/architecture.md`
§5 compares anarlog (sqlx, runtime queries, commit-hook signals), Handy
(rusqlite) and spacedrive (SeaORM).

## Decision

- **SQLite, one file:** `<app_data_dir>/deino.sqlite`, opened only by
  `deino_db::Database`: WAL, `foreign_keys = ON`, a 5 s busy timeout, **one
  writer connection** and a read pool of four `query_only` connections.
  Closing checkpoints the WAL.
- **sqlx 0.8 with `sqlx::migrate!`:** migrations live in
  `crates/deino-db/migrations/`, are append-only and are named with the real
  UTC time. They moved there from the app crate byte for byte, so existing
  databases keep their checksums.
- **Compile-checked queries:** repositories use `sqlx::query*!` with the
  `.sqlx/` offline data committed and `SQLX_OFFLINE=true` in
  `.cargo/config.toml`; `scripts/sqlx-prepare.sh` regenerates it. Queries that
  a test inspects with `EXPLAIN QUERY PLAN` live in `.sql` files.
- **Change signals:** hooks on the writer record the tables each transaction
  touches and publish them once on commit (cleared on rollback), adapted from
  anarlog's `db-change`. The app forwards them as `DbChanged` (ADR 0008).
- **Tests** run against `test_db()`: in memory, one connection, every
  migration, foreign keys on.

## Alternatives

- **Runtime queries only (anarlog):** no offline data to maintain, but typos
  and type drift surface only in tests.
- **SeaORM (spacedrive):** an ORM layer and entities that fight the
  one-function-per-query repository style.
- **rusqlite (Handy):** synchronous, and no migration tooling at our scale.
- **A pool of writers:** SQLite serialises writers anyway; one writer makes
  write order explicit and lets every commit pass the hooks.

## Consequences

- Legacy code writes through the same writer, so its writes refresh new
  queries too; a long legacy transaction delays other writes (it always did
  at the SQLite level).
- Changing a query or schema needs `scripts/sqlx-prepare.sh` (sqlx-cli 0.8).
- The update hook misses `DELETE` without `WHERE` and `REPLACE` conflicts;
  repositories avoid both for tracked tables.
