/**
 * TanStack Query hooks for the post-meeting pipeline.
 *
 * The status query names `jobs`, so `DbChanged` keeps it fresh after any
 * write; `usePipelineProgress` also refetches it the moment Rust reports a
 * stage change. A re-run writes the stages it returns into the cache.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { pipelineApi } from './api';
import type { MeetingId, StageId } from './types';

/** The tables the pipeline's queries read. */
const PIPELINE_TABLES = ['jobs'] as const satisfies readonly Table[];

/** Query keys; every key starts with `pipelineKeys.all`. */
export const pipelineKeys = {
  all: ['pipeline'] as const,
  status: (meetingId: MeetingId) => [...pipelineKeys.all, 'status', meetingId] as const,
};

/** A meeting's pipeline stages, in run order. */
export const pipelineStatusQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: pipelineKeys.status(meetingId),
    queryFn: () => pipelineApi.status(meetingId),
    meta: { tables: PIPELINE_TABLES },
  });

/** A meeting's pipeline stages, in run order. */
export function usePipelineStatus(meetingId: MeetingId) {
  return useQuery(pipelineStatusQuery(meetingId));
}

/** Re-runs one stage of a meeting (or all of them, with `null`). */
export function useRerunStage(meetingId: MeetingId) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (stage: StageId | null) => pipelineApi.rerun(meetingId, stage),
    onSuccess: (stages) => {
      client.setQueryData(pipelineKeys.status(meetingId), stages);
    },
  });
}
