/**
 * Auto-stop's "Ending…" notice and the "Start a new note?" offer (CAP-07),
 * as Rust shows them: read once, then kept current by `auto-stop-changed`.
 * They are Rust state, not a table, so `DbChanged` never covers them.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useIpcEvent } from '@/lib/ipc';
import { events } from '@/lib/ipc/generated/recording';

import { recordingApi } from '../api';
import { recordingCopy } from '../copy';
import { recordingKeys, report } from '../queries';
import type { AutoStopState } from '../types';

/** Nothing is shown. */
export const NOTHING_SHOWN: AutoStopState = { ending: null, split: null };

/** The notice and the offer on screen; events keep it current. */
export const autoStopQuery = () =>
  queryOptions({
    queryKey: recordingKeys.autoStop(),
    queryFn: recordingApi.autoStop,
    staleTime: Number.POSITIVE_INFINITY,
  });

/** What auto-stop shows now, following its events. */
export function useAutoStop(): AutoStopState {
  const client = useQueryClient();
  useIpcEvent(events.autoStopChanged, ({ state }) => {
    client.setQueryData(recordingKeys.autoStop(), state);
  });
  return useQuery(autoStopQuery()).data ?? NOTHING_SHOWN;
}

/** "Keep recording" and the answers to the offer. */
export function useAutoStopAnswers() {
  const client = useQueryClient();
  const remember = (state: AutoStopState) => {
    client.setQueryData(recordingKeys.autoStop(), state);
  };
  return {
    keep: useMutation({
      mutationFn: recordingApi.keepRecording,
      onSuccess: remember,
      onError: report(recordingCopy.stopFailed),
    }),
    split: useMutation({
      mutationFn: recordingApi.answerSplit,
      onSuccess: remember,
      onError: report(recordingCopy.startFailed),
    }),
  };
}
