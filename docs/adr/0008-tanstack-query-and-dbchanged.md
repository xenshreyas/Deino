# 0008. Cache Rust data in TanStack Query, invalidated by `DbChanged`

- **Status:** Accepted
- **Date:** 2026-10-08
- **Card:** FND-13 (`docs/references/architecture.md` §5, §7)

## Context

The UI kept Rust data in React contexts and refreshed it by hand
(`refetchMeetings()` after a save, local `setMeetings` after a rename). Any
write the UI did not know about, such as a summary renaming a meeting, left
stale data on screen. anarlog solves freshness with SQLite commit hooks and
live queries.

## Decision

- **TanStack Query owns all Rust data in the UI.** Each feature has
  `queries.ts` with key factories, `queryOptions` and hooks; `api.ts` is the
  only code that calls generated commands.
- **Every query declares the tables it reads** in `meta.tables`
  (typed through TanStack's `Register` as `readonly Table[]`).
- **Rust reports every committed write** as `DbChanged { tables, seq }`
  (`plugin:db:db-changed`), from the writer's commit hook (ADR 0005). The
  bridge in `lib/ipc/events.ts` invalidates every query whose tables
  intersect; active queries refetch. If the bridge lags behind, it reports all
  tables.
- **Defaults:** `staleTime: Infinity` and no refetch on focus, because the
  events already say when data changes; retries only for `unavailable` and
  `network` errors; `defaultError` is `IpcError`.
- **Mutations** write their result into the cache for an instant update; the
  `DbChanged` that follows reconciles.
- **Feature events** (`tauri_specta::Event`, read with `useIpcEvent`) are for
  domain moments the UI reacts to, not for cache freshness.

## Alternatives

- **Per-feature change events:** every feature must remember to emit, and a
  write from another feature or legacy code is missed.
- **Polling:** wasteful and still stale between polls.
- **Live queries (anarlog `db-reactive`):** precise, but a large dependency
  and SQL-level coupling in the UI.
- **Registering query keys per table in a central map:** a second list to keep
  in sync; `meta.tables` sits next to the query it describes.

## Consequences

- A new table needs a `Table` variant; nothing else is needed for freshness.
- Writes outside the hooked writer (another process, the `sqlite3` CLI) are
  invisible until the next invalidation or restart.
- Invalidation is per table, so a busy table refetches more than strictly
  needed; that is cheap for a single-user local database.
