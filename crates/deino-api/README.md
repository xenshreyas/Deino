# deino-api

## Role

The local REST API (DEV-03, ADR `2026-10-12-local-rest-api`): an `axum`
server on `127.0.0.1` only, off by default, that lets the user's own
scripts read and change their library with an API key from Settings →
Developer. Reads return what `deino --json` and the MCP tools return;
writes go through the app's services, so the UI updates at once.

## Owns

- `server.rs`: `RunningServer` (bind, serve, graceful stop) and
  `bind_address`, the only address it ever binds (loopback).
- `controller.rs`: `ApiController`, which starts, stops and moves the
  server with `settings.api`; `ApiStatus` for Settings.
- `routes.rs`, `routes/`: the `/v1` router and its handlers (meetings,
  transcripts, import, search, action items, tags, people, companies,
  folders, calendar events, `auth/test`).
- `guard.rs`: the middleware every route passes: `Host`, key, scope, lock;
  the request log.
- `page.rs`: opaque cursors tied to their filters, `Page`, the `Link`
  header.
- `error.rs`: `ApiError` and the `{"error": {"code", "message"}}` envelope.
- `state.rs`: `ApiServices` (the read layer, the redactor, the keys, the
  lock, `WriteServices`), `LockProbe`.

## Does Not Own

- Keys: `deino-core::api_keys` (argon2id hashes in `api_keys`).
- What a meeting reads as: `deino-core::query` (ADR 0035).
- The import's rules: `deino-core::transcript_import`.
- Wiring and the Settings commands: the app's `composition/api.rs` and
  `features/api`.

## Invariants

- Binds `127.0.0.1` only (`tests/server.rs` checks the bound address and
  `bind_address` for every port); answers only `Host: 127.0.0.1`,
  `localhost` or `[::1]`; sends no CORS headers.
- Never builds an HTTP client (ADR 0019); `hyper` is not a direct
  dependency.
- No SQL here: reads through `Library`, writes through the app's services.
- Every request but an unknown route needs a live key; a read-only key may
  only `GET`; while Deino is locked every request is `423`.
- Logs carry the method, the route pattern, the status and the key ID:
  never a body, a query string or a token.
