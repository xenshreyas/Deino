/**
 * TanStack Query hooks for ElevenLabs transcription (CAP-14).
 *
 * The key lives in the Keychain, so its status is not in a table: a change
 * writes the new status into the cache. Whether a meeting kept its
 * original transcript reads `original_transcripts`, which `DbChanged`
 * keeps fresh. A re-transcription's steps reach the processing strip
 * through the pipeline's own progress events and `DbChanged` on `jobs`.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Table } from '@/lib/ipc/generated/shared';

import { cloudTranscriptionApi } from './api';
import type { MeetingId, SecretStatus } from './types';

/** The tables the meeting queries read. */
const ORIGINAL_TABLES = ['original_transcripts'] as const satisfies readonly Table[];

/** Query keys; every key starts with `cloudKeys.all`. */
export const cloudKeys = {
  all: ['cloud-transcription'] as const,
  key: () => [...cloudKeys.all, 'key'] as const,
  original: (meetingId: MeetingId) => [...cloudKeys.all, 'original', meetingId] as const,
};

/** Whether a key is saved; re-read on each mount. */
export const elevenLabsKeyQuery = () =>
  queryOptions({
    queryKey: cloudKeys.key(),
    queryFn: cloudTranscriptionApi.keyStatus,
    staleTime: 0,
  });

/** Whether `meetingId` kept the transcript made on this Mac. */
export const originalTranscriptQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: cloudKeys.original(meetingId),
    queryFn: () => cloudTranscriptionApi.hasOriginal(meetingId),
    meta: { tables: ORIGINAL_TABLES },
  });

/** The key's status. */
export function useElevenLabsKey() {
  return useQuery(elevenLabsKeyQuery());
}

/** Whether the meeting can go back to its local transcript. */
export function useHasOriginalTranscript(meetingId: MeetingId) {
  return useQuery(originalTranscriptQuery(meetingId));
}

/** Writes `status` as the key's status. */
function useSetKeyStatus() {
  const client = useQueryClient();
  return (status: SecretStatus) => client.setQueryData(cloudKeys.key(), status);
}

/** Saves the key. */
export function useSaveElevenLabsKey() {
  const setStatus = useSetKeyStatus();
  return useMutation({
    mutationFn: cloudTranscriptionApi.saveKey,
    onSuccess: () => setStatus('stored'),
  });
}

/** Forgets the key. */
export function useForgetElevenLabsKey() {
  const setStatus = useSetKeyStatus();
  return useMutation({
    mutationFn: cloudTranscriptionApi.forgetKey,
    onSuccess: () => setStatus('missing'),
  });
}

/** Asks ElevenLabs about the saved key. */
export function useCheckElevenLabsKey() {
  return useMutation({ mutationFn: cloudTranscriptionApi.checkKey });
}

/** Re-transcribes a meeting with ElevenLabs. */
export function useRetranscribe(meetingId: MeetingId) {
  return useMutation({ mutationFn: () => cloudTranscriptionApi.retranscribe(meetingId) });
}

/** Restores a meeting's local transcript. */
export function useRestoreOriginal(meetingId: MeetingId) {
  return useMutation({ mutationFn: () => cloudTranscriptionApi.restoreOriginal(meetingId) });
}
