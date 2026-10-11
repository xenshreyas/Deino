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

/**
 * One meeting's notes. The editor reads them once when it mounts and owns
 * the document after that, so a refetch never moves the caret; it is kept
 * fresh for other readers (a second window).
 */
export const meetingNotesQuery = (id: MeetingId) =>
  queryOptions({
    queryKey: notesKeys.meeting(id),
    queryFn: () => notesApi.get(id),
    meta: { tables: NOTES_TABLES },
  });

/** One meeting's notes. */
export function useMeetingNotes(id: MeetingId) {
  return useQuery(meetingNotesQuery(id));
}
