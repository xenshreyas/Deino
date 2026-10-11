/**
 * TanStack Query over the transcript. The query names every table a
 * transcript is drawn from, so the recorder's batched saves, diarization,
 * speaker renames and people edits all refresh it through `DbChanged`.
 */
import { queryOptions } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { transcriptApi } from './api';
import type { MeetingId } from './types';

const TRANSCRIPT_TABLES = [
  'transcript_segments',
  'speakers',
  'people',
] as const satisfies readonly Table[];

/** Query keys; every key starts with `transcriptKeys.all`. */
export const transcriptKeys = {
  all: ['transcript'] as const,
  meeting: (meetingId: MeetingId) => [...transcriptKeys.all, meetingId] as const,
};

/** A meeting's saved transcript and speakers. */
export const transcriptQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: transcriptKeys.meeting(meetingId),
    queryFn: () => transcriptApi.get(meetingId),
    meta: { tables: TRANSCRIPT_TABLES },
  });
