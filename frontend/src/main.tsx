/**
 * UI entry point: loads global styles and fonts, then mounts the router.
 *
 * React StrictMode stays off, as it was under Next.js, where
 * `reactStrictMode: false` was set for BlockNote compatibility. NOT-01
 * revisits this when TipTap replaces BlockNote.
 */
import '@fontsource/source-sans-3/400.css';
import '@fontsource/source-sans-3/500.css';
import '@fontsource/source-sans-3/600.css';
import '@fontsource/source-sans-3/700.css';
import './styles/globals.css';

import { RouterProvider } from '@tanstack/react-router';
import { createRoot } from 'react-dom/client';

import { createQueryClient, IpcQueryProvider } from '@/lib/ipc';

import { router } from './router';

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

/** The app's one query cache for Rust data, refreshed by `DbChanged` events. */
const queryClient = createQueryClient();

createRoot(rootElement).render(
  <IpcQueryProvider client={queryClient}>
    <RouterProvider router={router} />
  </IpcQueryProvider>,
);
