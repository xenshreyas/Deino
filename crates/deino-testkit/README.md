# deino-testkit

## Role

Shared test fixtures and fakes. A dev-dependency only; never part of the app.

## Owns

- Re-exports with one import path: `test_db` (from `deino-db`) and `FakeClock`
  (from `deino-domain`).
- `at("2026-10-08T09:00:00Z")`: readable timestamps.
- `seed`: inserts rows no service can create yet (meetings, transcript chunks
  as the legacy recording path writes them).
- Later (FND-04): loaders for `fixtures/meetings/*.json` and fixture audio.

## Does Not Own

- Fakes of a crate's own ports: they live next to the trait, behind that
  crate's `test-support` feature (`FakeMeetingRepo` in `deino-core`).

## Invariants

- Depends only on `deino-domain` and `deino-db`, never on `deino-core`:
  core's tests use this crate, and a cycle would compile two copies of core's
  types.
- Helpers panic on bad fixtures: a broken fixture must fail the calling test.
