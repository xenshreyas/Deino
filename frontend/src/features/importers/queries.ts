/**
 * TanStack Query hooks for Settings → Import.
 *
 * Keys live in the Keychain and the job in memory, so neither is in a
 * table: a change writes its result into the cache, and the job's
 * `import-progress-changed` event replaces the progress as it moves. The
 * imported meetings reach every other list through `DbChanged`.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useIpcEvent } from '@/lib/ipc/events';
import { events } from '@/lib/ipc/generated/importers';

import { importersApi } from './api';
import type { ImportSource, ImportTarget } from './types';

/** Query keys; every key starts with `importerKeys.all`. */
export const importerKeys = {
  all: ['importers'] as const,
  sources: () => [...importerKeys.all, 'sources'] as const,
  progress: () => [...importerKeys.all, 'progress'] as const,
};

/** Every source and its key status; re-read on each mount. */
export const importSourcesQuery = () =>
  queryOptions({ queryKey: importerKeys.sources(), queryFn: importersApi.sources, staleTime: 0 });

/** The job's latest progress. */
export const importProgressQuery = () =>
  queryOptions({ queryKey: importerKeys.progress(), queryFn: importersApi.status, staleTime: 0 });

/** The sources. */
export function useImportSources() {
  return useQuery(importSourcesQuery());
}

/** The job's progress, kept current by its event. */
export function useImportProgress() {
  const client = useQueryClient();
  useIpcEvent(events.importProgressChanged, ({ progress }) => {
    client.setQueryData(importerKeys.progress(), progress);
  });
  return useQuery(importProgressQuery());
}

/** Re-reads the sources after a key changes. */
function useRefreshSources() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: importerKeys.sources() });
}

/** Saves a key. */
export function useSaveImportKey() {
  const onSuccess = useRefreshSources();
  return useMutation({
    mutationFn: ({ source, key }: { source: ImportSource; key: string }) =>
      importersApi.saveKey(source, key),
    onSuccess,
  });
}

/** Forgets a key. */
export function useForgetImportKey() {
  const onSuccess = useRefreshSources();
  return useMutation({
    mutationFn: (source: ImportSource) => importersApi.forgetKey(source),
    onSuccess,
  });
}

/** Opens the folder dialog. */
export function usePickImportFolder() {
  return useMutation({ mutationFn: importersApi.pickFolder });
}

/** Counts what an import would add (it asks the provider, so it is an action). */
export function usePreviewImport() {
  return useMutation({ mutationFn: (target: ImportTarget) => importersApi.preview(target) });
}

/** Starts an import; its first progress goes straight into the cache. */
export function useStartImport() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (target: ImportTarget) => importersApi.start(target),
    onSuccess: (progress) => {
      client.setQueryData(importerKeys.progress(), progress);
    },
  });
}

/** Stops the running import. */
export function useCancelImport() {
  return useMutation({ mutationFn: importersApi.cancel });
}
