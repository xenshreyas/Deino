/**
 * TanStack Query over the transcript. The query names `transcript_segments`,
 * so the recorder's batched saves, diarization and segment renames refresh
 * it through `DbChanged`; speaker names come from CAP-04's participants
 * query, which follows `speakers` and `people`.
 */
import { queryOptions } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { transcriptApi } from './api';
import type { MeetingId } from './types';

const TRANSCRIPT_TABLES = ['transcript_segments'] as const satisfies readonly Table[];

/** Query keys; every key starts with `transcriptKeys.all`. */
export const transcriptKeys = {
  all: ['transcript'] as const,
  meeting: (meetingId: MeetingId) => [...transcriptKeys.all, meetingId] as const,
};

/** A meeting's saved transcript. */
export const transcriptQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: transcriptKeys.meeting(meetingId),
    queryFn: () => transcriptApi.get(meetingId),
    meta: { tables: TRANSCRIPT_TABLES },
  });
