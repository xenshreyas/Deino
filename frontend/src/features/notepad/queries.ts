/**
 * TanStack Query hooks for the notepad. `meeting_notes` writes (the
 * autosave, a fixture reseed, an adopted draft) refresh them through
 * `DbChanged`.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';

import type { MeetingId } from '@/features/meetings';
import type { Table } from '@/lib/ipc/generated/shared';

import { notesApi } from './api';

/** The tables the notes query reads. */
const NOTES_TABLES = ['meeting_notes'] as const satisfies readonly Table[];

/** Query keys; every key starts with `notesKeys.all`. */
export const notesKeys = {
  all: ['notes'] as const,
  meeting: (id: MeetingId) => [...notesKeys.all, 'meeting', id] as const,
};

/** Retries of a meeting that is not there yet, a moment apart. */
const NOT_YET_RETRIES = 4;
const NOT_YET_DELAY_MS = 500;

/**
 * One meeting's notes. The editor reads them once when it mounts and owns
 * the document after that, so a refetch never moves the caret; it is kept
 * fresh for other readers (a second window). A meeting being recorded is
 * announced a moment before it is saved, so `not_found` is retried briefly.
 */
export const meetingNotesQuery = (id: MeetingId) =>
  queryOptions({
    queryKey: notesKeys.meeting(id),
    queryFn: () => notesApi.get(id),
    meta: { tables: NOTES_TABLES },
    retry: (failures, error) =>
      ['not_found', 'unavailable', 'network'].includes(error.code) && failures < NOT_YET_RETRIES,
    retryDelay: NOT_YET_DELAY_MS,
  });

/** One meeting's notes. */
export function useMeetingNotes(id: MeetingId) {
  return useQuery(meetingNotesQuery(id));
}
