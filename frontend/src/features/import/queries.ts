/**
 * TanStack Query hooks for imports and watched folders.
 *
 * The folder list and the failure list name their tables, so `DbChanged`
 * refreshes them when a background scan imports or fails a file. Mutations
 * also invalidate the import queries, so a change shows at once.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { importApi } from './api';
import type { WatchedFileId, WatchedFolderId } from './types';

/** The tables the folder list is read from. */
const FOLDER_TABLES = ['watched_folders'] as const satisfies readonly Table[];
/** The tables the failure list is read from. */
const FAILURE_TABLES = ['watched_files'] as const satisfies readonly Table[];

/** Query keys; every key starts with `importKeys.all`. */
export const importKeys = {
  all: ['import'] as const,
  folders: () => [...importKeys.all, 'folders'] as const,
  failures: () => [...importKeys.all, 'failures'] as const,
  check: (path: string) => [...importKeys.all, 'check', path] as const,
};

/** Every watched folder. */
export const watchedFoldersQuery = () =>
  queryOptions({
    queryKey: importKeys.folders(),
    queryFn: importApi.watchedFolders,
    meta: { tables: FOLDER_TABLES },
  });

/** Files watched folders could not import. */
export const watchFailuresQuery = () =>
  queryOptions({
    queryKey: importKeys.failures(),
    queryFn: importApi.failures,
    meta: { tables: FAILURE_TABLES },
  });

/** Every watched folder. */
export function useWatchedFolders() {
  return useQuery(watchedFoldersQuery());
}

/** Files watched folders could not import. */
export function useWatchFailures() {
  return useQuery(watchFailuresQuery());
}

/** A dropped file, checked; idle while `path` is `null`. */
export function useCheckedFile(path: string | null) {
  return useQuery({
    queryKey: importKeys.check(path ?? ''),
    queryFn: () => importApi.check(path ?? ''),
    enabled: path !== null,
    retry: false,
    staleTime: Infinity,
  });
}

/** Refetches every import query once a mutation settles. */
function useInvalidate() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: importKeys.all });
}

/** Opens the file dialog. */
export function usePickFile() {
  return useMutation({ mutationFn: importApi.pickFile });
}

/** Imports a file. */
export function useStartImport() {
  return useMutation({
    mutationFn: ({ path, title }: { path: string; title: string | null }) =>
      importApi.start(path, title),
  });
}

/** Opens the folder dialog and watches the chosen folder. */
export function useAddWatchedFolder() {
  const onSettled = useInvalidate();
  return useMutation({ mutationFn: importApi.addWatchedFolder, onSettled });
}

/** Stops watching a folder. */
export function useRemoveWatchedFolder() {
  const onSettled = useInvalidate();
  return useMutation({
    mutationFn: (id: WatchedFolderId) => importApi.removeWatchedFolder(id),
    onSettled,
  });
}

/** Turns "Move to processed" on or off. */
export function useSetMoveToProcessed() {
  const onSettled = useInvalidate();
  return useMutation({
    mutationFn: ({ id, on }: { id: WatchedFolderId; on: boolean }) =>
      importApi.setMoveToProcessed(id, on),
    onSettled,
  });
}

/** Retries (`retry`) or dismisses a failed file. */
export function useResolveFailure() {
  const onSettled = useInvalidate();
  return useMutation({
    mutationFn: ({ id, action }: { id: WatchedFileId; action: 'retry' | 'dismiss' }) =>
      action === 'retry' ? importApi.retry(id) : importApi.dismiss(id),
    onSettled,
  });
}
