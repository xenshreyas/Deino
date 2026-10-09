// The "-" prefix keeps the router plugin from treating this file as a route.
import { createMemoryHistory, createRouter } from '@tanstack/react-router';
import { describe, expect, test, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';
import { routeTree } from '@/routeTree.gen';

// Loaders only warm the cache; keep them away from Tauri.
vi.mock('@/lib/ipc/generated/meetings', () => ({
  commands: {
    list: vi.fn(() => Promise.resolve({ status: 'ok', data: [] })),
    get: vi.fn(() =>
      Promise.resolve({ status: 'error', error: { code: 'not_found', message: '' } }),
    ),
    rename: vi.fn(),
  },
}));

/** Resolves `url` through the real route tree without rendering anything. */
async function resolve(url: string) {
  const router = createRouter({
    routeTree,
    context: { queryClient: createQueryClient() },
    history: createMemoryHistory({ initialEntries: [url] }),
  });
  await router.load();
  const leaf = router.state.matches.at(-1);
  return { pathname: router.state.location.pathname, search: leaf?.search };
}

describe('route URLs', () => {
  test('the pre-Vite /meeting-details?id= URL redirects to the meeting route', async () => {
    const { pathname } = await resolve('/meeting-details?id=meeting-123');
    expect(pathname).toBe('/meetings/meeting-123');
  });

  test('the legacy redirect keeps the recording source so auto-summary still runs', async () => {
    const { pathname, search } = await resolve('/meeting-details?id=meeting-123&source=recording');
    expect(pathname).toBe('/meetings/meeting-123');
    expect(search).toEqual({ source: 'recording' });
  });

  test('the legacy URL without an id goes home instead of an error page', async () => {
    const { pathname } = await resolve('/meeting-details');
    expect(pathname).toBe('/');
  });

  test('an unknown source is dropped rather than failing the meeting route', async () => {
    const { pathname, search } = await resolve('/meetings/meeting-123?source=elsewhere');
    expect(pathname).toBe('/meetings/meeting-123');
    expect(search).toEqual({ source: undefined, tab: undefined });
  });

  test('the meeting tab lives in the URL so a link can open the transcript', async () => {
    const { search } = await resolve('/meetings/meeting-123?tab=transcript');
    expect(search).toEqual({ source: undefined, tab: 'transcript' });
  });

  test('an unknown tab falls back to the default instead of failing the route', async () => {
    const { search } = await resolve('/meetings/meeting-123?tab=nope');
    expect(search).toEqual({ source: undefined, tab: undefined });
  });

  test('every sidebar destination resolves to a route', async () => {
    const { NAV_ITEMS } = await import('@/features/shell');
    for (const item of NAV_ITEMS) {
      const { pathname } = await resolve(item.to);
      expect(pathname).toBe(item.to);
    }
  });
});
