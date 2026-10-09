/**
 * TanStack Query hooks for meeting detection.
 *
 * The prompt on screen is Rust state, not a table, so `DbChanged` never
 * covers it: {@link usePromptDetection} reads it once with `current` and
 * then follows the plugin's events, which write straight into the cache.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useIpcEvent } from '@/lib/ipc/events';
import { events } from '@/lib/ipc/generated/detection';

import { detectionApi } from './api';
import type { Detection, DetectionId, RecordingRequested } from './types';

/** Query keys; every key starts with `detectionKeys.all`. */
export const detectionKeys = {
  all: ['detection'] as const,
  current: () => [...detectionKeys.all, 'current'] as const,
  overview: () => [...detectionKeys.all, 'overview'] as const,
};

/** The prompt on screen, kept current by the detection events. */
export const currentDetectionQuery = () =>
  queryOptions({
    queryKey: detectionKeys.current(),
    queryFn: detectionApi.current,
    staleTime: Number.POSITIVE_INFINITY,
  });

/** The detection settings and what this Mac supports (Accessibility can change). */
export const detectionOverviewQuery = () =>
  queryOptions({
    queryKey: detectionKeys.overview(),
    queryFn: detectionApi.overview,
    staleTime: 0,
  });

/** The prompt to show, or `null`: `current` once, then every event. */
export function usePromptDetection(): Detection | null {
  const client = useQueryClient();
  const set = (detection: Detection | null) => {
    client.setQueryData(detectionKeys.current(), detection);
  };
  useIpcEvent(events.meetingDetected, ({ detection }) => {
    set(detection);
  });
  useIpcEvent(events.meetingEnded, () => {
    set(null);
  });
  useIpcEvent(events.promptClosed, () => {
    set(null);
  });
  return useQuery(currentDetectionQuery()).data ?? null;
}

/** The prompt's three answers; each closes the prompt in Rust. */
export type PromptAnswer = 'takeNotes' | 'dismiss' | 'ignoreApp';

/** Answers prompt `id`; the prompt clears itself as soon as it is sent. */
export function useAnswerPrompt() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ id, answer }: { id: DetectionId; answer: PromptAnswer }) =>
      detectionApi[answer](id),
    onSuccess: () => {
      client.setQueryData(detectionKeys.current(), null);
    },
  });
}

/** The detection settings and what this Mac supports. */
export function useDetectionOverview() {
  return useQuery(detectionOverviewQuery());
}

/** Calls `onRequest` whenever detection asks the main window to start recording. */
export function useRecordingRequests(onRequest: (request: RecordingRequested) => void): void {
  useIpcEvent(events.recordingRequested, onRequest);
}
