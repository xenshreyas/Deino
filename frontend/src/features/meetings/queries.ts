/**
 * TanStack Query hooks for meetings.
 *
 * Every query names the tables it reads in `meta.tables`, so `DbChanged`
 * refreshes it after any write, including legacy code that saves a recording.
 * Mutations write their result into the cache for an instant update; the
 * `DbChanged` that follows reconciles.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { meetingsApi } from './api';
import type { Meeting, MeetingId, RenameMeetingInput } from './types';

/** The tables every meetings query reads. */
const MEETING_TABLES = ['meetings'] as const satisfies readonly Table[];

/** Query keys; every key starts with `meetingKeys.all`. */
export const meetingKeys = {
  all: ['meetings'] as const,
  list: () => [...meetingKeys.all, 'list'] as const,
  detail: (id: MeetingId) => [...meetingKeys.all, 'detail', id] as const,
};

/** Every meeting, newest first. Also usable from route loaders. */
export const meetingListQuery = () =>
  queryOptions({
    queryKey: meetingKeys.list(),
    queryFn: meetingsApi.list,
    meta: { tables: MEETING_TABLES },
  });

/** One meeting. */
export const meetingQuery = (id: MeetingId) =>
  queryOptions({
    queryKey: meetingKeys.detail(id),
    queryFn: () => meetingsApi.get(id),
    meta: { tables: MEETING_TABLES },
  });

/** Every meeting, newest first. */
export function useMeetings() {
  return useQuery(meetingListQuery());
}

/** One meeting. */
export function useMeeting(id: MeetingId) {
  return useQuery(meetingQuery(id));
}

/** Renames a meeting and updates the cached list and detail at once. */
export function useRenameMeeting() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ id, title }: RenameMeetingInput) => meetingsApi.rename(id, title),
    onSuccess: (renamed) => {
      client.setQueryData(meetingKeys.detail(renamed.id), renamed);
      client.setQueryData(meetingKeys.list(), (meetings: Meeting[] | undefined) =>
        meetings?.map((meeting) => (meeting.id === renamed.id ? renamed : meeting)),
      );
    },
  });
}
