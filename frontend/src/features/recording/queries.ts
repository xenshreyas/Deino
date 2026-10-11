/**
 * TanStack Query over the recorder.
 *
 * The state query is fed by `recording-state` events (`RecordingSync`
 * writes each snapshot into the cache), so it never refetches on its own.
 * A meeting's saved transcript names `transcript_segments`, so the
 * recorder's batched saves refresh it through `DbChanged`.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import type { MeetingId } from '@/features/meetings';
import { IpcError } from '@/lib/ipc';
import type { Table } from '@/lib/ipc/generated/shared';

import { recordingApi } from './api';
import { recordingCopy } from './copy';
import type { RecordingSnapshot } from './types';

const TRANSCRIPT_TABLES = ['transcript_segments'] as const satisfies readonly Table[];

/** Query keys; every key starts with `recordingKeys.all`. */
export const recordingKeys = {
  all: ['recording'] as const,
  state: () => [...recordingKeys.all, 'state'] as const,
  transcript: (meetingId: MeetingId) => [...recordingKeys.all, 'transcript', meetingId] as const,
};

/** Nothing is being recorded (also what shows before the first answer). */
export const IDLE: RecordingSnapshot = {
  phase: 'idle',
  meetingId: null,
  title: null,
  startedAt: null,
  pausedMs: 0,
  pausedAt: null,
  echo: null,
};

/** The recorder's state; events keep it current. */
export const recordingStateQuery = () =>
  queryOptions({
    queryKey: recordingKeys.state(),
    queryFn: recordingApi.state,
    staleTime: Infinity,
  });

/** A meeting's saved transcript, oldest first. */
export const recordingTranscriptQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: recordingKeys.transcript(meetingId),
    queryFn: () => recordingApi.transcript(meetingId),
    meta: { tables: TRANSCRIPT_TABLES },
  });

/** The recorder's state now; idle until Rust answers. */
export function useRecordingState(): RecordingSnapshot {
  return useQuery(recordingStateQuery()).data ?? IDLE;
}

/** Shows a failed recorder command; `conflict` means it already happened. */
function report(title: string) {
  return (error: unknown) => {
    const failure = IpcError.from(error);
    if (failure.code === 'conflict') return;
    toast.error(title, { description: failure.message });
  };
}

/** Starts a recording from anywhere (sidebar, ⌘⇧R, palette); errors become toasts. */
export function startRecording(title: string | null = null): Promise<void> {
  return recordingApi.start(title).then(() => undefined, report(recordingCopy.startFailed));
}

/** Start, pause, resume and stop, each answering with the new state. */
export function useRecordingActions() {
  const client = useQueryClient();
  const remember = (snapshot: RecordingSnapshot) => {
    client.setQueryData(recordingKeys.state(), snapshot);
  };
  return {
    start: useMutation({ mutationFn: () => startRecording() }),
    pause: useMutation({
      mutationFn: recordingApi.pause,
      onSuccess: remember,
      onError: report(recordingCopy.pauseFailed),
    }),
    resume: useMutation({
      mutationFn: recordingApi.resume,
      onSuccess: remember,
      onError: report(recordingCopy.pauseFailed),
    }),
    stop: useMutation({ mutationFn: recordingApi.stop, onError: report(recordingCopy.stopFailed) }),
  };
}
