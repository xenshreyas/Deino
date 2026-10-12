/**
 * Home's own IPC: opening an upcoming event's note, which creates its
 * scheduled meeting (NOT-01) and links it to the event (ORG-03), so notes
 * can be written before the call.
 */
import type { CalendarEvent } from '@/features/calendar';
import { eventLinksApi } from '@/features/event-links';
import type { MeetingId } from '@/features/meetings';
import { commands as notes } from '@/lib/ipc/generated/notes';
import { unwrap } from '@/lib/ipc/result';

export const homeApi = {
  /** Creates `event`'s note, links it, and resolves to the meeting. */
  openEventNote: async (event: CalendarEvent): Promise<MeetingId> => {
    const meeting = await unwrap(notes.createScheduled(event.title));
    await eventLinksApi.link(meeting.id, event.id);
    return meeting.id;
  },
};
