# deino-core

## Role

Deino's features as Tauri-free modules: the rules, the ports they need and
the SQLite adapters behind those ports.

## Owns

- `meetings`: `MeetingService` (list, get, rename), the `MeetingRepo` port,
  `SqliteMeetingRepo` and `FakeMeetingRepo`.
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
| `queries/*.sql` | Queries a test inspects with `EXPLAIN QUERY PLAN` |

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
- Fakes honour their port's whole contract, ordering included.
