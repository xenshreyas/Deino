/**
 * Event linking and related meetings (ORG-03), through the `calendar`
 * plugin's linking commands. Other code imports from here only.
 *
 * - `MeetingCalendarChips`: the meeting header's event chip with its picker
 *   (suggestions from the day before the meeting, link, re-link, unlink;
 *   also mid-recording) and the "Previous" chip;
 * - `MeetingEventChip`: the event chip alone (the record screen);
 * - `MeetingLinksPanel`: the Details panel's linked event, "Previous in
 *   series" and related meetings.
 */
export { eventLinksApi } from './api';
export { MeetingCalendarChips, PreviousMeetingChip } from './components/meeting-calendar-chips';
export { MeetingEventChip, type MeetingEventChipProps } from './components/meeting-event-chip';
export { MeetingLinksPanel } from './components/meeting-links-panel';
export {
  eventLinkKeys,
  linkedEventQuery,
  relatedMeetingsQuery,
  useLinkedEvent,
  useRelatedMeetings,
} from './queries';
export type { EventLinkChange, RelatedBy, RelatedMeeting, RelatedMeetings } from './types';
