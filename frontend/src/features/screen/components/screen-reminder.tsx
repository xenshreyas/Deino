/**
 * The screen capture consent reminder under the recording pill (CAP-09):
 * CAP-13's reminder card, shown the first three times a capture runs
 * (counted in Rust, `ConsentReminders`). "Got it" hides it for this
 * meeting; "Don't show again" ends the reminders. The record panel shows
 * it, folded into the consent reminder when that one shows too (UX-28).
 */
import { useState } from 'react';

import type { ReminderState } from '@/features/consent';

import {
  useScreenReminder as useReminderQuery,
  useScreenStatus,
  useStopScreenReminders,
} from '../queries';
import type { CaptureStatus, MeetingId } from '../types';

/** `meetingId` while its screen is being captured, else `null`. */
export function capturedMeeting(
  capture: CaptureStatus | undefined,
  meetingId: MeetingId | null,
): MeetingId | null {
  const captured =
    capture !== undefined && capture.state !== 'off' && capture.meeting === meetingId;
  return captured ? meetingId : null;
}

/** The reminder's state, plus whether the screen is being captured at all. */
export interface ScreenReminderState extends ReminderState {
  captured: boolean;
}

/** The screen reminder's state for the recording of `meetingId`. */
export function useScreenReminder(meetingId: MeetingId | null): ScreenReminderState {
  const status = useScreenStatus();
  const captured = capturedMeeting(status.data?.capture, meetingId);
  const remind = useReminderQuery(captured);
  const stop = useStopScreenReminders();
  const [dismissedFor, setDismissedFor] = useState<MeetingId | null>(null);
  return {
    captured: captured !== null,
    shown: captured !== null && remind.data === true && dismissedFor !== captured,
    busy: stop.isPending,
    dismiss: () => {
      setDismissedFor(captured);
    },
    never: () => {
      setDismissedFor(captured);
      stop.mutate(undefined, {
        onError: (error) => {
          console.error('[screen] Could not turn the reminder off', error);
        },
      });
    },
  };
}
