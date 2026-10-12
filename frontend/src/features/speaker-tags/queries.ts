/**
 * TanStack Query hooks for speaker tags.
 *
 * The status is live macOS state, not a table, so `DbChanged` never covers
 * it: the page polls it while it is open, which is how a permission granted
 * in System Settings shows up here without a reload.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';

import { speakerTagsApi } from './api';

/** How often the page re-reads the status while it is open. */
export const STATUS_POLL_MS = 2_000;

/** Query keys; every key starts with `speakerTagKeys.all`. */
export const speakerTagKeys = {
  all: ['speaker-tags'] as const,
  status: () => [...speakerTagKeys.all, 'status'] as const,
};

/** Each meeting app's speaker-tag state. */
export const speakerTagStatusQuery = () =>
  queryOptions({
    queryKey: speakerTagKeys.status(),
    queryFn: speakerTagsApi.status,
    staleTime: 0,
    refetchInterval: STATUS_POLL_MS,
  });

/** Each meeting app's speaker-tag state, kept fresh while mounted. */
export function useSpeakerTagStatus() {
  return useQuery(speakerTagStatusQuery());
}
