/**
 * Tells Rust which calendar event's note is on screen (ORG-02), so the
 * meeting ticker can start recording it at its start time when the
 * auto-record rules say so. Renders nothing; mounted on the meeting page.
 */
import { useLinkedEvent } from '@/features/event-links';
import type { MeetingId } from '@/features/meetings';
import { useMountEffect } from '@/hooks/useMountEffect';

import { remindersApi } from '../reminders';
import type { CalendarEventId } from '../types';

/** Reports `eventId` while mounted; a new event remounts it (`key`). */
function Report({ eventId }: { eventId: CalendarEventId }) {
  useMountEffect(() => {
    void remindersApi.setOpenNote(eventId).catch(() => undefined);
    return () => {
      void remindersApi.setOpenNote(null).catch(() => undefined);
    };
  });
  return null;
}

/** Reports the meeting's linked event while its page is open. */
export function OpenNoteReporter({ meetingId }: { meetingId: MeetingId }) {
  const event = useLinkedEvent(meetingId);
  const eventId = event.data?.id;
  return eventId === undefined ? null : <Report key={eventId} eventId={eventId} />;
}
