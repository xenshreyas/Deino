/**
 * The app's TanStack Router.
 *
 * Routes are file-based: `@tanstack/router-plugin` generates `routeTree.gen.ts`
 * from `src/routes/` on every dev and build run (ADR 0003). Every route gets
 * the query client in its context, so loaders can warm the cache with
 * `context.queryClient.prefetchQuery(...)` before the screen renders.
 */
import type { QueryClient } from '@tanstack/react-query';
import { createRouter } from '@tanstack/react-router';

import { routeTree } from './routeTree.gen';

/** What every route's `beforeLoad` and `loader` receive as `context`. */
export interface RouterContext {
  queryClient: QueryClient;
}

/** Creates the router over the app's query client. */
export function createAppRouter(queryClient: QueryClient) {
  return createRouter({
    routeTree,
    context: { queryClient },
    defaultPreload: 'intent',
    // Query data is already cached by TanStack Query; the router keeps none.
    defaultPreloadStaleTime: 0,
  });
}

declare module '@tanstack/react-router' {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}
