/**
 * TanStack Query hooks for briefs. Queries name the `briefs` table, so
 * `DbChanged` refreshes them when the background job writes a new brief or
 * the user reads one.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { CalendarEventId } from '@/lib/ipc/generated/calendar';
import type { MeetingId } from '@/lib/ipc/generated/meetings';
import type { Table } from '@/lib/ipc/generated/shared';

import { briefsApi } from './api';
import type { BriefFeedback } from './types';

/** The tables a brief reads (a meeting's brief follows its calendar link). */
const BRIEF_TABLES = ['briefs', 'meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `briefKeys.all`. */
export const briefKeys = {
  all: ['briefs'] as const,
  event: (event: CalendarEventId) => [...briefKeys.all, 'event', event] as const,
  meeting: (meeting: MeetingId) => [...briefKeys.all, 'meeting', meeting] as const,
};

/** The brief of an upcoming event: what ORG-02's event note shows. */
export function useEventBrief(event: CalendarEventId) {
  return useQuery({
    queryKey: briefKeys.event(event),
    queryFn: () => briefsApi.forEvent(event),
    meta: { tables: BRIEF_TABLES },
  });
}

/** The brief of the event a meeting note is linked to. */
export function useMeetingBrief(meeting: MeetingId) {
  return useQuery({
    queryKey: briefKeys.meeting(meeting),
    queryFn: () => briefsApi.forMeeting(meeting),
    meta: { tables: BRIEF_TABLES },
  });
}

/** Refetches every brief query once a mutation settles. */
function useInvalidateBriefs() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: briefKeys.all });
}

/** Marks a brief read, so it collapses. */
export function useMarkBriefRead() {
  const invalidate = useInvalidateBriefs();
  return useMutation({ mutationFn: briefsApi.markRead, onSettled: invalidate });
}

/** Stores the thumbs down on a brief. */
export function useBriefFeedback() {
  const invalidate = useInvalidateBriefs();
  return useMutation({
    mutationFn: ({ event, feedback }: { event: CalendarEventId; feedback: BriefFeedback }) =>
      briefsApi.giveFeedback(event, feedback),
    onSettled: invalidate,
  });
}
