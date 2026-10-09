/**
 * UI entry point: loads the font and global styles, applies the saved theme,
 * then mounts the router.
 *
 * React StrictMode stays off. It was off for BlockNote, which NOT-01 replaced
 * with TipTap (StrictMode-safe); the legacy contexts still start recordings,
 * listeners and recovery checks in mount effects that are not idempotent, so
 * turning it on waits until they are ported.
 */
import '@fontsource-variable/inter';
import './styles/globals.css';

import { RouterProvider } from '@tanstack/react-router';
import { createRoot } from 'react-dom/client';

import { bootTheme } from '@/features/appearance';
import { createQueryClient, IpcQueryProvider } from '@/lib/ipc';

import { createAppRouter } from './router';

// `pnpm dev:mock`: run in a plain browser on fixture data. Production builds
// drop this branch, and with it everything under `src/dev/`.
if (import.meta.env.MODE === 'mock') {
  const { installMockIpc } = await import('@/dev/mock-ipc/install');
  installMockIpc();
}

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('index.html is missing the #root element');
}

bootTheme();

/** The app's one query cache for Rust data, refreshed by `DbChanged` events. */
const queryClient = createQueryClient();
const router = createAppRouter(queryClient);

createRoot(rootElement).render(
  <IpcQueryProvider client={queryClient}>
    <RouterProvider router={router} />
  </IpcQueryProvider>,
);
