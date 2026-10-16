/**
 * TanStack Query hooks for the post-meeting pipeline.
 *
 * The status query names `jobs`, so `DbChanged` keeps it fresh after any
 * write; `usePipelineProgress` also refetches it the moment Rust reports a
 * stage change. A re-run writes the stages it returns into the cache.
 *
 * A running stage's percent is never fetched: `usePipelineProgress` writes
 * the latest one from `PipelineProgress` into the cache, per meeting and
 * stage, and clears it on the stage's next status change.
 */
import {
  queryOptions,
  skipToken,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import { toast } from 'sonner';

import type { Table } from '@/lib/ipc/generated/shared';

import { pipelineApi } from './api';
import { copy } from './copy';
import type { MeetingId, StageId } from './types';

/** The tables the pipeline's queries read. */
const PIPELINE_TABLES = ['jobs'] as const satisfies readonly Table[];

/** Query keys; every key starts with `pipelineKeys.all`. */
export const pipelineKeys = {
  all: ['pipeline'] as const,
  status: (meetingId: MeetingId) => [...pipelineKeys.all, 'status', meetingId] as const,
  percent: (meetingId: MeetingId, stage: StageId) =>
    [...pipelineKeys.all, 'percent', meetingId, stage] as const,
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

/**
 * How far a running stage is, 0–100, from the last `PipelineProgress` that
 * said so; null when it hasn't said. Cache-only: nothing is fetched.
 */
export const stagePercentQuery = (meetingId: MeetingId, stage: StageId) =>
  queryOptions<number | null>({
    queryKey: pipelineKeys.percent(meetingId, stage),
    queryFn: skipToken,
    staleTime: Infinity,
  });

/** How far `stage` of the meeting is, 0–100, or null when unknown. */
export function useStagePercent(meetingId: MeetingId, stage: StageId): number | null {
  return useQuery(stagePercentQuery(meetingId, stage)).data ?? null;
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

/**
 * Re-transcribes a meeting and re-runs the steps after it. The status line
 * shows the run, so success is silent; a failure is a toast (UX-32).
 */
export function useRetranscribe(meetingId: MeetingId) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: () => pipelineApi.retranscribe(meetingId),
    onSuccess: (stages) => {
      client.setQueryData(pipelineKeys.status(meetingId), stages);
    },
    onError: (error) => {
      toast.error(copy.retranscribeFailed, { description: error.message });
    },
  });
}
