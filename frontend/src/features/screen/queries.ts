/**
 * TanStack Query hooks for screen capture.
 *
 * The status is live macOS state, not a table, so the page polls it while
 * it is open (a permission granted in System Settings shows up without a
 * reload). Snapshots are the `snapshots` table, which `DbChanged` keeps
 * fresh. The consent reminder is asked once per captured meeting.
 */
import {
  queryOptions,
  skipToken,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';

import type { MeetingId } from '@/lib/ipc/generated/screen';

import { screenApi } from './api';

/** How often the status is re-read while it is shown. */
export const STATUS_POLL_MS = 2_000;

/** Query keys; every key starts with `screenKeys.all`. */
export const screenKeys = {
  all: ['screen'] as const,
  status: () => [...screenKeys.all, 'status'] as const,
  snapshots: (meetingId: MeetingId) => [...screenKeys.all, 'snapshots', meetingId] as const,
  folder: (meetingId: MeetingId) => [...screenKeys.all, 'folder', meetingId] as const,
  reminder: (meetingId: MeetingId | null) => [...screenKeys.all, 'reminder', meetingId] as const,
};

/** Screen capture on this Mac, kept fresh while mounted. */
export const screenStatusQuery = () =>
  queryOptions({
    queryKey: screenKeys.status(),
    queryFn: screenApi.status,
    staleTime: 0,
    refetchInterval: STATUS_POLL_MS,
  });

/** A meeting's snapshots. */
export const snapshotsQuery = (meetingId: MeetingId) =>
  queryOptions({
    queryKey: screenKeys.snapshots(meetingId),
    queryFn: () => screenApi.snapshots(meetingId),
    meta: { tables: ['snapshots'] },
  });

/** Screen capture on this Mac. */
export function useScreenStatus() {
  return useQuery(screenStatusQuery());
}

/** Opens the Screen Recording page of System Settings. */
export function useOpenScreenRecordingSettings() {
  return useMutation({ mutationFn: screenApi.openScreenRecordingSettings });
}

/** "Turn off for this meeting". */
export function useTurnOffForMeeting() {
  return useMutation({ mutationFn: screenApi.turnOffForMeeting });
}

/** A meeting's snapshots, kept fresh by `DbChanged`. */
export function useSnapshots(meetingId: MeetingId) {
  return useQuery(snapshotsQuery(meetingId));
}

/** The folder of a meeting's snapshot PNG files (fixed for the meeting). */
export function useSnapshotFolder(meetingId: MeetingId) {
  return useQuery({
    queryKey: screenKeys.folder(meetingId),
    queryFn: () => screenApi.snapshotFolder(meetingId),
    staleTime: Infinity,
  });
}

/** Deletes a snapshot; the list refreshes right away. */
export function useDeleteSnapshot() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: screenApi.deleteSnapshot,
    onSuccess: (_deleted, { meetingId }) =>
      client.invalidateQueries({ queryKey: screenKeys.snapshots(meetingId) }),
  });
}

/** Whether the capture of `meetingId` shows the consent reminder; asked
 * once per meeting, and only while it is being captured. */
export function useScreenReminder(meetingId: MeetingId | null) {
  return useQuery({
    queryKey: screenKeys.reminder(meetingId),
    queryFn: meetingId === null ? skipToken : () => screenApi.consentReminder(meetingId),
    staleTime: Infinity,
  });
}

/** "Don't show again" on the screen capture reminder. */
export function useStopScreenReminders() {
  return useMutation({ mutationFn: screenApi.stopConsentReminders });
}
