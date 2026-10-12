/**
 * Test helper: renders a settings section at `/settings` with a query
 * client, tooltips and a router, so sections with links (the permission
 * link, cross-page links) render as they do in the app.
 */
import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { render } from '@testing-library/react';
import type { ComponentType } from 'react';

import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

/** Renders `Section` as the `/settings` route; returns the router. */
export function renderInSettings(Section: ComponentType, url = '/settings') {
  const root = createRootRoute();
  const settings = createRoute({
    getParentRoute: () => root,
    path: '/settings',
    component: () => (
      <TooltipProvider>
        <Section />
      </TooltipProvider>
    ),
  });
  const router = createRouter({
    routeTree: root.addChildren([settings]),
    history: createMemoryHistory({ initialEntries: [url] }),
  });
  render(
    <QueryClientProvider client={createQueryClient()}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  return router;
}
