# deino-domain

## Role

Deino's shared vocabulary: the types that must mean the same thing in every
crate and across IPC. No I/O, no Tauri.

## Owns

- Typed IDs (`id.rs`): `MeetingId` (legacy `meeting-<uuid>` text) and UUIDv7
  ID newtypes declared with `uuid_id!`, each serde-, sqlx- and
  specta-transparent.
- The IPC error model (`error.rs`): `AppError { code, message }`, `ErrorCode`,
  and the `ErrorClass` trait every module error implements (ADR 0009).
- The `Clock` port (`clock.rs`): `SystemClock`, and `FakeClock` behind the
  `test-support` feature.

## Does Not Own

- Feature models and rules: `deino-core`.
- Database access or the `Table` enum: `deino-db`.
- Anything Tauri: the app crate.

## Invariants

- No dependency on `tauri`, `deino-db` or `deino-core`.
- New entity IDs are declared in `id.rs` with `uuid_id!` and generated from a
  `Clock`, never `Uuid::now_v7()`.
- `ErrorCode` is a closed set; adding a code changes the bindings and the
  UI's handling, so it needs a reason in the PR.
- `AppError::message` is user-safe English: never meeting content.
- `SystemClock` is the only code allowed to read the wall clock.
