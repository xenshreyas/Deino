# The local REST API: loopback axum over the app's services, hashed keys, refused while locked

- **Status:** Accepted
- **Date:** 2026-10-12
- **Card:** DEV-03 (local REST API); builds on ADR 0035 (the read layer),
  ADR 0046 (the trash), ADR 0019 (network egress) and ADR 0050 (the app lock)

## Context

DEV-03 asks for Circleback's REST API (C §12.1) on the Mac: keys that read
or read and write, cursor pagination, an import endpoint, and the usual
meetings, search, action items, tags, people, companies, folders and
calendar routes. Deino is local-first and single-user; nothing may listen
on the network, the UI must see every write, and the app lock (PRV-02)
must mean something for a process that can read the library.

Questions: where the server lives and what it binds; how keys are kept;
where reads and writes go; what the lock does to it; how lists page.

## Decision

1. **A new crate, `deino-api`, with `axum` 0.8.9** (default features off:
   `http1`, `json`, `query`, `matched-path`, `tokio`; no tracing, no
   HTTP/2). It only answers; it has no HTTP client, so ADR 0019's "only
   `deino-net` sends" rule holds (`check-dependency-rules.sh` still
   passes: axum is a server and `hyper` is not a direct dependency).
2. **Loopback only.** `bind_address(port)` is always `127.0.0.1:<port>`;
   there is no setting for the interface. The server is **off by default**
   (`settings.api.enabled`) on port 47810 (`settings.api.port`, 1024 or
   higher). `ApiController` (the `api` plugin's `configure_server`) saves
   the setting and starts, stops or moves the server; a plain settings patch
   of `api.*` is refused, so the switch and the server cannot disagree. At
   startup the controller starts it if the setting is on; a taken port is
   reported in Settings, not fatal.
3. **A `Host` check.** Requests whose `Host` is not `127.0.0.1`,
   `localhost` or `[::1]` on the server's port get `403 forbidden_host`, so
   a web page that rebinds its own domain to 127.0.0.1 cannot reach the
   API. No CORS headers are sent, so browsers cannot attach a key
   cross-origin.
4. **Keys are hashed with argon2id.** Token `deino_<key id>_<64 hex>`
   (256 random bits from the OS). The `api_keys` table keeps the name, the
   scope (`read` or `read_write`), times, and the PHC string of an
   argon2id hash (crate defaults, 19 MiB, two passes, a salt per key). The
   token is returned once by `create_key` and never stored or logged. The
   key ID in the token finds the row, so a request costs one hash check
   (off the async workers). Missing, malformed, unknown, wrong and revoked
   keys all get the same `401 unauthorized`. `last_used_at` is written at
   most once a minute. A read-only key may only `GET` (`403 read_only_key`).
5. **Reads go through `deino-core::query`** (ADR 0035), the CLI's read
   layer, so `/v1` returns the same objects as `deino --json` and the MCP
   tools (`MeetingSummary`, `MeetingDetail`, `TranscriptLine`,
   `ActionItemSummary`, `PersonView`, ...). Redaction (NOT-09) is the
   exports' `SettingRedactor` with patterns only, as for the CLI; the API
   also redacts search snippets.
6. **Writes go through the app's own services**, never SQL in a handler:
   `MeetingService::rename`, `NoteService::save` (my notes from Markdown),
   `TagService` (create, update, delete, apply, remove), `TrashService`
   (`DELETE /v1/meetings/{id}` moves to the trash, ADR 0046),
   `ActionItemService`, and the new `TranscriptImporter` for
   `POST /v1/meetings` (Circleback's import shape, through the repositories
   the sample importer uses, then the pipeline's re-index). The services
   are the managed instances, so writes take the single writer, fire their
   events and reach the UI through `DbChanged`.
7. **Locked means closed.** While `LockService::is_locked()`, every `/v1`
   request (after the key check) gets `423 locked`, reads and writes alike.
   The lock protects the library from anyone who can reach the window
   (ADR 0050); a local script with a key is such a caller, so it waits for
   the unlock like the window does. (The read-only `deino` CLI is a
   separate process that opens the file itself and is not affected; that
   is ADR 0035's trade-off and unchanged.)
8. **Cursor pagination as Circleback does it:** `{"data", "nextCursor"}`
   plus an RFC 8288 `Link: <...>; rel="next"` header. Cursors are opaque
   base64url JSON carrying a fingerprint of the filters; a cursor sent with
   other filters is `400 invalid`. Meetings page by key (the last meeting's
   `(created_at, id)` in the list's order, `MeetingQueries::page`), so a
   meeting added or trashed between pages neither repeats nor skips one.
   The small lists (people, companies, tags, folders, events, action items,
   search hits) page by position.
9. **One error model:** `{"error": {"code", "message"}}`, the MCP tools'
   shape, with `AppError`'s `ErrorCode` (snake case) mapped to a status
   (`not_found` 404, `invalid` 400, `conflict` 409, `unavailable` 503,
   `internal` 500, ...) plus `unauthorized`, `read_only_key`, `locked` and
   `forbidden_host`. Bad JSON and bad query strings use it too.
10. **Logs carry no bodies:** one line per request with the method, the
    route pattern (`/v1/meetings/{id}`, never the query string), the status
    and the key's ID.

## Alternatives

- **Serve on `0.0.0.0` behind a setting.** Exposes meetings to the LAN for
  no use case the owner has; a phone app would need TLS and pairing anyway.
- **Store keys in the Keychain.** Verification needs only a hash, and a
  hash in SQLite cannot leak a usable key; the Keychain would prompt.
- **A fast hash (SHA-256) instead of argon2.** Tokens have 256 bits of
  entropy, so a fast hash would be safe too; argon2id is what the card asks
  for and costs about 10 ms a request, fine at local rates.
- **Read through the writer pool or raw SQL in handlers.** Breaks the
  "the API returns what the CLI returns" rule and duplicates rules (names,
  trash, deleted lines).
- **Serve reads while locked.** The lock would then hide the library from
  a person at the Mac but not from a script on it; refusing everything is
  simpler to reason about, and a script can retry after a 423.
- **Offset pagination for meetings.** New meetings arrive at the top of
  the list, so offsets would repeat items across pages.

## Consequences

- A new table, `api_keys`, and a new crate in the workspace; `argon2`
  0.5.3 joins `deino-core`, `axum` 0.8.9 and `base64` 0.22 `deino-api`.
- Search through the API is keyword-only until the app's embedder is
  shared with it (a follow-up); the reply's hits say nothing about mode.
- Not built yet (follow-ups): `PUT /v1/calendar/events/{id}/notes`,
  `POST /v1/recording/start|stop`, the OpenAPI 3.1 document (`utoipa`) at
  `/v1/openapi.json`, and the 20 requests a second rate limit.
- `include=transcript` inlines up to 20,000 lines; longer transcripts are
  read page by page from `/v1/meetings/{id}/transcript`.
