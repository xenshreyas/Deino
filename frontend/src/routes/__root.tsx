import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';

import { LibraryGate } from '@/features/privacy';
import { AppShell } from '@/features/shell';

import type { RouterContext } from '../router';

export const Route = createRootRouteWithContext<RouterContext>()({ component: RootLayout });

/**
 * Every route renders inside the app shell (providers, sidebar, onboarding
 * gate), once the library is open (PRV-02: it may be converting first).
 */
function RootLayout() {
  return (
    <LibraryGate>
      <AppShell>
        <Outlet />
      </AppShell>
    </LibraryGate>
  );
}
