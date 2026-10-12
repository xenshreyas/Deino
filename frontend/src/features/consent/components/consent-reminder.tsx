/**
 * The consent reminder under the recording pill (CAP-13): shown once per
 * recorded meeting while `consent.reminder` is on, until "Got it" or "Don't
 * show again". {@link useConsentReminder} is its state, so the record panel
 * can fold the screen capture reminder into the same card (UX-28);
 * {@link ConsentReminder} is the card on its own.
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

/** A reminder card's state and answers. */
export interface ReminderState {
  shown: boolean;
  /** "Don't show again" is being saved. */
  busy: boolean;
  /** "Got it": hides it for this meeting. */
  dismiss: () => void;
  /** "Don't show again": turns it off. */
  never: () => void;
}

/** The consent reminder's state for the recording of `meetingId` (`null`: nothing records). */
export function useConsentReminder(meetingId: string | null): ReminderState {
  const reminder = useReminder(meetingId);
  const stop = useStopReminding();
  const [dismissedFor, setDismissedFor] = useState<string | null>(null);
  return {
    shown: shouldRemind(reminder.data, meetingId, dismissedFor),
    busy: stop.isPending,
    dismiss: () => {
      setDismissedFor(meetingId);
    },
    never: () => {
      setDismissedFor(meetingId);
      stop.mutate(undefined, {
        onError: (error) => {
          console.error('[consent] Could not turn the reminder off', error);
        },
      });
    },
  };
}

/** Props of {@link ConsentReminder}. */
export interface ConsentReminderProps {
  /** The meeting being recorded; `null` while nothing records. */
  meetingId: string | null;
}

/** The reminder for the recording of `meetingId`, while it is wanted. */
export function ConsentReminder({ meetingId }: ConsentReminderProps) {
  const reminder = useConsentReminder(meetingId);
  if (!reminder.shown) return null;
  return <ConsentCard busy={reminder.busy} onDismiss={reminder.dismiss} onNever={reminder.never} />;
}
