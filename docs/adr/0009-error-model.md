# 0009. Report errors as `AppError { code, message }` from typed module errors

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13 (`docs/references/architecture.md` §3)

## Context

Inherited commands return `Result<T, String>`; the UI cannot tell "not found"
from "database broken" and shows raw error text, which sometimes contains
user content. None of the Tauri references types its IPC errors. PARITY_PLAN
§3A asks for `thiserror` enums per module, no `String` errors in the core and
`anyhow` only at the edges.

## Decision

- **Module errors** are `thiserror` enums, `#[non_exhaustive]`, one doc
  comment per variant saying when it happens, with context fields and
  `#[source]` (zed's style, reimplemented). Their `Display` text is user-safe:
  IDs and limits, never meeting content.
- **`ErrorClass`** (in `deino-domain`) maps each variant to an `ErrorCode`:
  `not_found`, `invalid`, `conflict`, `unavailable`, `permission_denied`,
  `network`, `llm`, `cancelled`, `internal`. The set is closed and exported to
  TypeScript with a runtime list (`ERROR_CODES`).
- **`AppError { code, message }`** is the only error that crosses IPC;
  `From<E: ErrorClass>` builds it.
- **The command boundary** (`IntoIpc::into_ipc`) is the only place that logs an
  error's whole source chain: at `warn` for failures (`internal`,
  `unavailable`, `network`, `llm`), at `debug` for refusals.
- **The UI** throws `IpcError` (code and message) from `unwrap()`. Errors
  Tauri raises itself (an ACL denial, a missing state) arrive as strings and
  become `internal`.

## Alternatives

- **Strings (anarlog, Cap, Handy):** no machine-readable failure kind.
- **A large error enum across the app:** couples every feature to every other
  feature's failures.
- **HTTP-like numeric codes:** less readable and no better for a local app.

## Consequences

- The UI branches on `code` (for example, show a field error for `invalid`).
- Adding a code is a deliberate change to `deino-domain` and the bindings.
- Module errors stay rich for logs and tests while IPC stays small.
