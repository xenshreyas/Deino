/**
 * Typed calls to the `calendar` plugin's linking commands (ORG-03): the only
 * place this feature touches IPC. Each resolves to the data or throws an
 * `IpcError`.
 */
import { commands } from '@/lib/ipc/generated/calendar';
import { unwrap } from '@/lib/ipc/result';

import type {
  CalendarEvent,
  CalendarEventId,
  EventLinkChange,
  MeetingId,
  RelatedMeetings,
} from './types';

export const eventLinksApi = {
  /** The event the meeting is linked to; `null` without one. */
  linked: (meetingId: MeetingId): Promise<CalendarEvent | null> =>
    unwrap(commands.meetingEvent(meetingId)),
  /** The picker's suggestions: the day before the meeting to an hour after, closest first. */
  suggestions: (meetingId: MeetingId): Promise<CalendarEvent[]> =>
    unwrap(commands.eventSuggestions(meetingId)),
  /** Links the meeting to an event (also mid-recording); re-linking replaces the attendees. */
  link: (meetingId: MeetingId, eventId: CalendarEventId): Promise<EventLinkChange> =>
    unwrap(commands.linkEvent(meetingId, eventId)),
  /** Unlinks the meeting from its event; the title stays. */
  unlink: (meetingId: MeetingId): Promise<EventLinkChange> =>
    unwrap(commands.unlinkEvent(meetingId)),
  /** Meetings of the same series, or with the same title and people. */
  related: (meetingId: MeetingId): Promise<RelatedMeetings> =>
    unwrap(commands.relatedMeetings(meetingId)),
};
