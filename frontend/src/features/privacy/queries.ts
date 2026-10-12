/**
 * TanStack Query over the app lock and the library status.
 *
 * The lock state is polled (and refetched when the window regains focus,
 * which is how a wake or screen lock shows up): an idle lock takes effect
 * in Rust the moment it is asked. The library status polls only while the
 * database is still opening or being converted.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import { IpcError } from '@/lib/ipc';

import { privacyApi } from './api';
import { privacyCopy } from './copy';
import type { LibraryStatus, LockState } from './types';

/** How often the lock state is asked while the window is open. */
export const LOCK_POLL_MS = 5_000;
/** How often the library status is asked while it is converting. */
export const LIBRARY_POLL_MS = 500;

/** Query keys; every key starts with `privacyKeys.all`. */
export const privacyKeys = {
  all: ['privacy'] as const,
  lock: () => [...privacyKeys.all, 'lock'] as const,
  library: () => [...privacyKeys.all, 'library'] as const,
};

/** True while the library is not ready yet but will be. */
function isPending(status: LibraryStatus | undefined): boolean {
  return status === undefined || status.kind === 'opening' || status.kind === 'converting';
}

/** Whether the library is open. */
export const libraryStatusQuery = () =>
  queryOptions({
    queryKey: privacyKeys.library(),
    queryFn: privacyApi.library,
    refetchInterval: (query) => (isPending(query.state.data) ? LIBRARY_POLL_MS : false),
  });

/** The library status; `undefined` until Rust answers. */
export function useLibraryStatus(): LibraryStatus | undefined {
  return useQuery(libraryStatusQuery()).data;
}

/** Locked or not. */
export const lockStateQuery = () =>
  queryOptions({
    queryKey: privacyKeys.lock(),
    queryFn: privacyApi.state,
    refetchInterval: LOCK_POLL_MS,
    refetchOnWindowFocus: 'always',
  });

/** The lock state; unlocked until Rust answers. */
export function useLockState(): LockState {
  return useQuery(lockStateQuery()).data ?? 'unlocked';
}

/** Unlock (shows the system prompt) and lock now; both update the cache. */
export function useLockActions() {
  const queryClient = useQueryClient();
  const store = (state: LockState) => {
    queryClient.setQueryData(privacyKeys.lock(), state);
  };
  const unlock = useMutation({
    mutationFn: privacyApi.unlock,
    onSuccess: store,
    onError: (error) => {
      const failure = IpcError.from(error);
      if (failure.code === 'cancelled') return;
      toast.error(privacyCopy.lock.failed, { description: failure.message });
    },
  });
  const lock = useMutation({ mutationFn: privacyApi.lock, onSuccess: store });
  return { unlock, lock };
}
