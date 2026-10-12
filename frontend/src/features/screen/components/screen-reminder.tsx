/**
 * The screen capture consent reminder under the recording pill (CAP-09):
 * CAP-13's reminder card, shown the first three times a capture runs
 * (counted in Rust, `ConsentReminders`). "Got it" hides it for this
 * meeting; "Don't show again" ends the reminders.
 */
import { useState } from 'react';

import { ConsentCard } from '@/features/consent';

import { screenCopy } from '../copy';
import { useScreenReminder, useScreenStatus, useStopScreenReminders } from '../queries';
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

/** The reminder for the capture of `meetingId`, while it is wanted. */
export function ScreenReminder({ meetingId }: { meetingId: MeetingId | null }) {
  const status = useScreenStatus();
  const captured = capturedMeeting(status.data?.capture, meetingId);
  const remind = useScreenReminder(captured);
  const stop = useStopScreenReminders();
  const [dismissedFor, setDismissedFor] = useState<MeetingId | null>(null);
  if (captured === null || remind.data !== true || dismissedFor === captured) return null;
  return (
    <ConsentCard
      copy={screenCopy.reminder}
      busy={stop.isPending}
      onDismiss={() => {
        setDismissedFor(captured);
      }}
      onNever={() => {
        setDismissedFor(captured);
        stop.mutate(undefined, {
          onError: (error) => {
            console.error('[screen] Could not turn the reminder off', error);
          },
        });
      }}
    />
  );
}
