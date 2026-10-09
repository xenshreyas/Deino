/**
 * The app's TanStack Router instance.
 *
 * Routes are file-based: `@tanstack/router-plugin` generates `routeTree.gen.ts`
 * from `src/routes/` on every dev and build run (ADR 0003).
 */
import { createRouter } from '@tanstack/react-router';

import { routeTree } from './routeTree.gen';

/** The single router used by `main.tsx`. */
export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
