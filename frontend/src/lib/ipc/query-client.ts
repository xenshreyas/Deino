/**
 * The app's TanStack Query setup for data that lives in Rust (ADR 0008).
 *
 * Rust owns the data and tells the UI when it changes (`DbChanged`, see
 * `events.ts`), so a cached result stays correct until an event says
 * otherwise: no timers, no refetch on focus. Queries declare the tables they
 * read in `meta.tables`; that is the whole invalidation contract.
 */
import { QueryClient } from '@tanstack/react-query';

import type { Table } from './generated/shared';
import type { IpcError } from './result';

/** Error codes worth retrying: the failure may be gone a moment later. */
const RETRYABLE_CODES = new Set<IpcError['code']>(['unavailable', 'network']);

/** How many times a retryable query is retried before it shows its error. */
const MAX_RETRIES = 2;

/** Metadata an IPC query carries. */
export interface IpcQueryMeta extends Record<string, unknown> {
  /** The tables whose changes make this query stale. */
  readonly tables?: readonly Table[];
}

declare module '@tanstack/react-query' {
  interface Register {
    defaultError: IpcError;
    queryMeta: IpcQueryMeta;
  }
}

/** Creates the query client the app (or a test) runs on. */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: Number.POSITIVE_INFINITY,
        refetchOnWindowFocus: false,
        retry: (failureCount, error) =>
          RETRYABLE_CODES.has(error.code) && failureCount < MAX_RETRIES,
      },
    },
  });
}
