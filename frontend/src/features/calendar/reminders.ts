/**
 * ORG-02's reminder and event-menu calls and hooks: the reminder window's
 * event (refreshed by `ReminderShown`) and dismiss, the open note the
 * start-time trigger watches, "Review last meeting" and "Open in Calendar".
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useIpcEvent } from '@/lib/ipc/events';
import { commands, events } from '@/lib/ipc/generated/calendar';
import type { CalendarEvent, CalendarEventId, RelatedMeeting } from '@/lib/ipc/generated/calendar';
import { unwrap } from '@/lib/ipc/result';

import { calendarKeys } from './queries';

export const remindersApi = {
  /** The event the reminder window is for, or `null` once it closed. */
  current: (): Promise<CalendarEvent | null> => unwrap(commands.reminder()),
  /** Closes the reminder window. */
  dismiss: (): Promise<null> => unwrap(commands.dismissReminder()),
  /** Tells Rust which event's note is on screen (`null`: none). */
  setOpenNote: (event: CalendarEventId | null): Promise<null> =>
    unwrap(commands.setOpenNote(event)),
  /** The meeting to review before `id`, if there is one. */
  lastMeeting: (id: CalendarEventId): Promise<RelatedMeeting | null> =>
    unwrap(commands.lastMeeting(id)),
  /** Opens `id` in Calendar, or its web calendar on the event's day. */
  openInCalendar: (id: CalendarEventId): Promise<null> => unwrap(commands.openInCalendar(id)),
};

/** The reminder window's event. */
export const reminderQuery = () =>
  queryOptions({
    queryKey: [...calendarKeys.all, 'reminder'] as const,
    queryFn: remindersApi.current,
    meta: { tables: ['calendar_events'] },
  });

/** The reminder window's event, refetched whenever the window is reused. */
export function useReminder() {
  const client = useQueryClient();
  useIpcEvent(events.reminderShown, () => {
    void client.invalidateQueries({ queryKey: reminderQuery().queryKey });
  });
  return useQuery(reminderQuery());
}

/** Closes the reminder window and clears its card until the next reminder. */
export function useDismissReminder() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: remindersApi.dismiss,
    onSuccess: () => {
      client.setQueryData(reminderQuery().queryKey, null);
    },
  });
}

/** "Review last meeting": resolves to the meeting, or `null`. */
export function useLastMeeting() {
  return useMutation({ mutationFn: remindersApi.lastMeeting });
}

/** "Open in Calendar". */
export function useOpenInCalendar() {
  return useMutation({ mutationFn: remindersApi.openInCalendar });
}

export type { RelatedMeeting };
