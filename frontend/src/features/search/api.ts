/**
 * Typed calls to the `search` plugin. Each resolves to the data or throws an
 * `IpcError`; components use the hooks in `queries.ts` instead.
 */
import { commands } from '@/lib/ipc/generated/search';
import { unwrap } from '@/lib/ipc/result';

import type { FilterChip, FilterKind, IndexStatus, SearchRequest, SearchResults } from './types';

export const searchApi = {
  /** Meetings, moments, people, companies, action items and folders, by section. */
  search: (request: SearchRequest): Promise<SearchResults> => unwrap(commands.search(request)),
  /** `@` filter candidates whose name starts with `prefix`. */
  suggestFilters: (prefix: string, kind: FilterKind | null = null): Promise<FilterChip[]> =>
    unwrap(commands.suggestFilters(prefix, kind)),
  /** The embedding model, its coverage and the backfill. */
  indexStatus: (): Promise<IndexStatus> => unwrap(commands.indexStatus()),
  /** Downloads the embedding model, then starts the backfill; resolves when done. */
  downloadModel: (): Promise<IndexStatus> => unwrap(commands.downloadModel()),
  /** Stops the model download, keeping what arrived. */
  cancelDownload: (): Promise<boolean> => unwrap(commands.cancelDownload()),
  /** Deletes the model file; search becomes keyword-only. */
  deleteModel: (): Promise<IndexStatus> => unwrap(commands.deleteModel()),
  /** Embeds every finished meeting not embedded yet. */
  startBackfill: (): Promise<IndexStatus> => unwrap(commands.startBackfill()),
  /** Rebuilds the keyword index from every source. */
  reindexAll: () => unwrap(commands.reindexAll()),
};
