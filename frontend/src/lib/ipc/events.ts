/**
 * Typed Rust events, and the `DbChanged` bridge to TanStack Query (ADR 0008).
 *
 * Rust reports every committed write with the tables it touched, and the
 * queries that read those tables refetch. A query opts in by naming its tables
 * in `meta.tables`; no feature ever invalidates after another feature's write.
 * Feature events (`events.<name>` in a generated file) are for domain moments
 * the UI reacts to; subscribe with {@link useIpcEvent}.
 */
import type { QueryClient } from '@tanstack/react-query';
import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event';
import { useRef } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { events } from './generated/db';
import type { Table } from './generated/shared';

/** A generated tauri-specta event, such as `events.noteAdded`. */
export interface IpcEvent<T> {
  listen: (handler: EventCallback<T>) => Promise<UnlistenFn>;
}

/** Marks every query that reads one of `changed` as stale; active ones refetch. */
export function invalidateTables(client: QueryClient, changed: readonly Table[]): Promise<void> {
  const tables = new Set(changed);
  return client.invalidateQueries({
    predicate: (query) => query.meta?.tables?.some((table) => tables.has(table)) ?? false,
  });
}

/** Invalidates queries on every `DbChanged` until the returned function is called. */
export function listenToDbChanges(client: QueryClient): () => void {
  return listen(events.dbChanged, ({ tables }) => {
    void invalidateTables(client, tables);
  });
}

/** Calls `onEvent` with each event's payload while the component is mounted. */
export function useIpcEvent<T>(event: IpcEvent<T>, onEvent: (payload: T) => void): void {
  // Always call the latest handler without re-subscribing on every render.
  const handler = useRef(onEvent);
  handler.current = onEvent;
  useMountEffect(() =>
    listen(event, (payload) => {
      handler.current(payload);
    }),
  );
}

/** Subscribes to `event`; the returned function unsubscribes, even before Tauri confirms. */
function listen<T>(event: IpcEvent<T>, onEvent: (payload: T) => void): () => void {
  const listening = event.listen(({ payload }) => {
    onEvent(payload);
  });
  return () => {
    void listening.then((unlisten) => {
      unlisten();
    });
  };
}
