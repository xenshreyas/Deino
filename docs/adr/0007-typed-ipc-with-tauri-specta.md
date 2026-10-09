# 0007. Generate typed IPC bindings with tauri-specta

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13 (`docs/references/architecture.md` §2)

## Context

The UI called about 150 commands as `invoke<T>('api_get_meetings')` with
hand-written TypeScript types, so a Rust change broke the UI only at runtime.
Every Tauri reference generates bindings with tauri-specta; none has typed
errors.

## Decision

- **Pins:** `tauri-specta =2.0.0-rc.21`, `specta =2.0.0-rc.22`,
  `specta-typescript =0.0.9` (Handy's and anarlog's set), upgraded only
  together. They resolve with Tauri 2.11.
- **One `tauri_specta::Builder` per feature plugin** (`features/<f>.rs`
  `specta()`), because `Builder::commands` overwrites earlier calls. The
  invoke handler and the bindings come from the same builder, so they cannot
  drift.
- **Commands stay private** and are collected in `commands.rs`
  (`collect()`): `#[tauri::command]` on a `pub` function exports a macro at the
  crate root, and two features with a `list` command would collide. Commands
  are not generic over the runtime, so the mock runtime can test them.
- **Errors:** commands return `Result<T, AppError>` (ADR 0009) with
  `ErrorHandlingMode::Result`; the UI's `unwrap()` turns them into `IpcError`.
- **Generation from a test:** `cargo test -p deino --lib bindings` writes
  `frontend/src/lib/ipc/generated/<feature>.ts` plus `shared.ts` (`AppError`,
  `ErrorCode`, `Table`, `ERROR_CODES`, `TABLES`). IDs are branded
  (`string & { readonly __brand: "MeetingId" }`). Files are written only when
  they change. The output is deterministic; CI regenerates and diffs it.
- **Guard tests:** the exported `plugin:<name>|<cmd>` set equals
  `features/manifest.rs`; every manifest feature is granted by a capability;
  each command has a contract test on the mock runtime.
- **Debug-only plugins** (`DEV_FEATURES`, `capabilities/dev/`) are registered
  by `build.rs` only when debug assertions are on.

## Alternatives

- **ts-rs:** types only, no command or event wrappers.
- **Hand-written types with zod validation:** runtime safety, but every
  command is typed twice.
- **One global builder:** impossible to compose from feature modules.
- **`tauri-specta-query`:** generates TanStack hooks, but is at v0.0.1.

## Consequences

- A Rust signature change fails the frontend typecheck, and typed binding
  mocks in tests (`satisfies typeof commands`).
- The generated files duplicate shared types per feature; they are
  structurally identical, so TypeScript treats them as one.
- tauri-specta is a release candidate; an upgrade is one PR that moves the
  three pins and regenerates.
