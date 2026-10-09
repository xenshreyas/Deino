import { createRootRoute, Outlet } from '@tanstack/react-router';

import { AppShell } from '@/features/shell';

export const Route = createRootRoute({ component: RootLayout });

/** Every route renders inside the app shell (providers, sidebar, onboarding gate). */
function RootLayout() {
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}
