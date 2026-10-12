/**
 * TanStack Query hooks for retention. The status reads `meetings`, so
 * `DbChanged` refreshes it the moment the daily sweep deletes something.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { retentionApi } from './api';
import { retentionCopy } from './copy';
import type { MeetingId } from './types';

/** The tables the status reads. */
const STATUS_TABLES = ['meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `retentionKeys.all`. */
export const retentionKeys = {
  all: ['retention'] as const,
  meeting: (meeting: MeetingId) => [...retentionKeys.all, 'meeting', meeting] as const,
};

/** What retention removed from `meeting`. */
export function meetingRetentionQuery(meeting: MeetingId) {
  return queryOptions({
    queryKey: retentionKeys.meeting(meeting),
    queryFn: () => retentionApi.status(meeting),
    meta: { tables: STATUS_TABLES },
  });
}

/** What retention removed from `meeting`, kept fresh. */
export function useMeetingRetention(meeting: MeetingId) {
  return useQuery(meetingRetentionQuery(meeting));
}

/**
 * Why the meeting's notes cannot be regenerated (its transcript was
 * deleted), or `null` when they can.
 */
export function useRegenerateBlock(meeting: MeetingId): string | null {
  const deletedAt = useMeetingRetention(meeting).data?.transcriptDeletedAt ?? null;
  return deletedAt === null ? null : retentionCopy.deleted.regenerateBlocked;
}
