import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';

import { AppShell } from '@/features/shell';

import type { RouterContext } from '../router';

export const Route = createRootRouteWithContext<RouterContext>()({ component: RootLayout });

/** Every route renders inside the app shell (providers, sidebar, onboarding gate). */
function RootLayout() {
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}
