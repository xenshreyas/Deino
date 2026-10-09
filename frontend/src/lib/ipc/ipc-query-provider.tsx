import { type QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';

import { useMountEffect } from '@/hooks/useMountEffect';

import { listenToDbChanges } from './events';

interface IpcQueryProviderProps {
  /** The app's client, from `createQueryClient()`. */
  client: QueryClient;
  children: ReactNode;
}

/**
 * Provides the query client and keeps it in sync with Rust: while mounted,
 * every `DbChanged` event invalidates the queries that read the changed tables.
 */
export function IpcQueryProvider({ client, children }: IpcQueryProviderProps) {
  useMountEffect(() => listenToDbChanges(client));
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
