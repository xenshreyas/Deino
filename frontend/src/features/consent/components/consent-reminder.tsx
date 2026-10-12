/**
 * The consent reminder under the recording pill (CAP-13): shown once per
 * recorded meeting while `consent.reminder` is on, until "Got it" or "Don't
 * show again". The record panel renders it with the meeting being recorded.
 */
import { useState } from 'react';

import { useReminder, useStopReminding } from '../queries';
import { ConsentCard } from './consent-card';

/** Whether the reminder shows for `meetingId`, given the setting and the
 * meeting it was last dismissed for. */
export function shouldRemind(
  reminder: boolean | undefined,
  meetingId: string | null,
  dismissedFor: string | null,
): boolean {
  return reminder === true && meetingId !== null && meetingId !== dismissedFor;
}

/** Props of {@link ConsentReminder}. */
export interface ConsentReminderProps {
  /** The meeting being recorded; `null` while nothing records. */
  meetingId: string | null;
}

/** The reminder for the recording of `meetingId`, while it is wanted. */
export function ConsentReminder({ meetingId }: ConsentReminderProps) {
  const reminder = useReminder(meetingId);
  const stop = useStopReminding();
  const [dismissedFor, setDismissedFor] = useState<string | null>(null);
  if (meetingId === null || !shouldRemind(reminder.data, meetingId, dismissedFor)) return null;
  return (
    <ConsentCard
      busy={stop.isPending}
      onDismiss={() => {
        setDismissedFor(meetingId);
      }}
      onNever={() => {
        setDismissedFor(meetingId);
        stop.mutate(undefined, {
          onError: (error) => {
            console.error('[consent] Could not turn the reminder off', error);
          },
        });
      }}
    />
  );
}
