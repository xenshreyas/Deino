/**
 * TanStack Query hooks for the consent reminder. The reminder setting lives
 * in the settings file, not a table, so it is read afresh for each
 * recording (the query key carries the meeting) rather than kept fresh by
 * `DbChanged`.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { speakerTagsApi } from '@/features/speaker-tags';

import { consentApi } from './api';

/** Query keys; every key starts with `consentKeys.all`. */
export const consentKeys = {
  all: ['consent'] as const,
  reminder: (meetingId: string) => [...consentKeys.all, 'reminder', meetingId] as const,
};

/** Whether to remind the user when `meetingId` starts recording. */
export const reminderQuery = (meetingId: string) =>
  queryOptions({
    queryKey: consentKeys.reminder(meetingId),
    queryFn: consentApi.reminder,
    staleTime: Infinity,
  });

/** The reminder setting for the recording of `meetingId`, if one runs. */
export function useReminder(meetingId: string | null) {
  return useQuery({ ...reminderQuery(meetingId ?? ''), enabled: meetingId !== null });
}

/** "Don't show again". */
export function useStopReminding() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: consentApi.stopReminding,
    onSuccess: () => client.invalidateQueries({ queryKey: consentKeys.all }),
  });
}

/** Opens System Settings → Privacy & Security → Accessibility. */
export function useOpenAccessibilitySettings() {
  return useMutation({ mutationFn: speakerTagsApi.openAccessibilitySettings });
}
