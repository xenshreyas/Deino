import type { EventCallback } from '@tauri-apps/api/event';
import { describe, expect, it, vi } from 'vitest';

import type { DbChanged } from './generated/db';
import { listenToDbChanges } from './events';
import { createQueryClient } from './query-client';

const dbChanged = vi.hoisted(() => {
  const state: { handler?: EventCallback<DbChanged> } = {};
  const unlisten = vi.fn();
  const listen = vi.fn((handler: EventCallback<DbChanged>) => {
    state.handler = handler;
    return Promise.resolve(unlisten);
  });
  return { state, listen, unlisten };
});
vi.mock('./generated/db', () => ({ events: { dbChanged: { listen: dbChanged.listen } } }));

/** Delivers a `DbChanged` event as Tauri would. */
function emit(payload: DbChanged): void {
  dbChanged.state.handler?.({ event: 'plugin:db:db-changed', id: 1, payload });
}

describe('listenToDbChanges', () => {
  it('invalidates the queries that read a changed table, and no others', async () => {
    const client = createQueryClient();
    const meetings = vi.fn(() => Promise.resolve([]));
    const settings = vi.fn(() => Promise.resolve({}));
    await client.query({
      queryKey: ['meetings'],
      queryFn: meetings,
      meta: { tables: ['meetings'] },
    });
    await client.query({
      queryKey: ['settings'],
      queryFn: settings,
      meta: { tables: ['settings'] },
    });

    const stop = listenToDbChanges(client);
    emit({ tables: ['meetings'], seq: 1 });

    expect(client.getQueryState(['meetings'])?.isInvalidated).toBe(true);
    expect(client.getQueryState(['settings'])?.isInvalidated).toBe(false);
    stop();
  });

  it('stops listening when the returned function is called', async () => {
    const stop = listenToDbChanges(createQueryClient());

    stop();

    await vi.waitFor(() => {
      expect(dbChanged.unlisten).toHaveBeenCalledOnce();
    });
  });
});
