/**
 * TanStack Query hooks for a meeting's thread. The thread names the
 * `ask_messages` table, so `DbChanged` refreshes it after any write.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { Table } from '@/lib/ipc/generated/shared';

import { askApi } from './api';

/** The tables a thread reads. */
const ASK_TABLES = ['ask_messages'] as const satisfies readonly Table[];

/** Query keys; every key starts with `askKeys.all`. */
export const askKeys = {
  all: ['ask'] as const,
  thread: (meeting: MeetingId) => [...askKeys.all, 'thread', meeting] as const,
};

/** The meeting's thread, oldest first. */
export function useAskThread(meeting: MeetingId) {
  return useQuery({
    queryKey: askKeys.thread(meeting),
    queryFn: () => askApi.thread(meeting),
    meta: { tables: ASK_TABLES },
  });
}

/** Empties the meeting's thread. */
export function useClearThread(meeting: MeetingId) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: () => askApi.clear(meeting),
    onSettled: () => client.invalidateQueries({ queryKey: askKeys.thread(meeting) }),
  });
}
