// The "-" prefix keeps the router plugin from treating this file as a route.
import { createMemoryHistory, createRouter } from '@tanstack/react-router';
import { describe, expect, test } from 'vitest';

import { routeTree } from '@/routeTree.gen';

/** Resolves `url` through the real route tree without rendering anything. */
async function resolve(url: string) {
  const router = createRouter({
    routeTree,
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
    expect(search).toEqual({ source: undefined });
  });
});
