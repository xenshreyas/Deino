/**
 * TanStack Query hooks for search.
 *
 * A search reads the index and the tables its filters join, so `DbChanged`
 * refreshes open results after any write (a new transcript line, a rename).
 * The index status follows the backfill's and the model download's events.
 */
import {
  keepPreviousData,
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/search';
import type { Table } from '@/lib/ipc/generated/shared';

import { searchApi } from './api';
import type { FilterKind, SearchRequest } from './types';

/** The tables a search reads: the index, meetings (trash) and the filters' joins. */
const SEARCH_TABLES = [
  'search_documents',
  'embeddings',
  'meetings',
  'meeting_attendees',
  'meeting_tags',
  'meeting_folders',
  'speakers',
] as const satisfies readonly Table[];
/** The tables `@` suggestions read. */
const FILTER_TABLES = [
  'people',
  'companies',
  'tags',
  'folders',
  'meetings',
] as const satisfies readonly Table[];
/** The tables the index status counts. */
const STATUS_TABLES = [
  'embeddings',
  'search_documents',
  'meetings',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `searchKeys.all`. */
export const searchKeys = {
  all: ['search'] as const,
  results: (request: SearchRequest) => [...searchKeys.all, 'results', request] as const,
  filters: (prefix: string, kind: FilterKind | null) =>
    [...searchKeys.all, 'filters', prefix, kind] as const,
  status: () => [...searchKeys.all, 'status'] as const,
};

/** The results of `request`. */
export const searchQuery = (request: SearchRequest) =>
  queryOptions({
    queryKey: searchKeys.results(request),
    queryFn: () => searchApi.search(request),
    meta: { tables: SEARCH_TABLES },
  });

/**
 * Results that keep showing the previous query's while the next one runs,
 * so typing never flashes an empty list.
 */
export function useSearch(request: SearchRequest, enabled: boolean) {
  return useQuery({ ...searchQuery(request), enabled, placeholderData: keepPreviousData });
}

/** `@` suggestions for what follows the `@`. */
export function useFilterSuggestions(prefix: string, enabled: boolean) {
  return useQuery({
    queryKey: searchKeys.filters(prefix, null),
    queryFn: () => searchApi.suggestFilters(prefix),
    meta: { tables: FILTER_TABLES },
    enabled,
    placeholderData: keepPreviousData,
  });
}

/** The embedding model, coverage and backfill, kept fresh by their events. */
export function useIndexStatus() {
  const client = useQueryClient();
  const refresh = () => void client.invalidateQueries({ queryKey: searchKeys.status() });
  useIpcEvent(events.indexProgress, refresh);
  useIpcEvent(events.embeddingModelChanged, refresh);
  return useQuery({
    queryKey: searchKeys.status(),
    queryFn: searchApi.indexStatus,
    meta: { tables: STATUS_TABLES },
  });
}

/** The model download's bytes so far, from its progress events (or `null`). */
export function useModelDownloadProgress(onProgress: (downloadedBytes: number) => void) {
  useIpcEvent(events.embeddingModelProgress, ({ progress }) => {
    onProgress(progress.downloadedBytes);
  });
}

/** One of the Search index page's actions, refreshing the status when it ends. */
function useIndexAction<T>(action: () => Promise<T>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: action,
    onSettled: () => client.invalidateQueries({ queryKey: searchKeys.status() }),
  });
}

/** Downloads the model (then the backfill starts on its own). */
export const useDownloadModel = () => useIndexAction(searchApi.downloadModel);
/** Stops the model download. */
export const useCancelDownload = () => useIndexAction(searchApi.cancelDownload);
/** Deletes the model file. */
export const useDeleteModel = () => useIndexAction(searchApi.deleteModel);
/** Starts the backfill. */
export const useStartBackfill = () => useIndexAction(searchApi.startBackfill);
/** Rebuilds the keyword index. */
export const useReindexAll = () => useIndexAction(searchApi.reindexAll);
