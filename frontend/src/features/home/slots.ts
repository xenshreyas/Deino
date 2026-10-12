/**
 * Typed slots other cards fill on Home and the event note. Each renders
 * nothing while it has no data.
 *
 * - `followUps` (NOT-07): suggested follow-ups, such as a recap to send.
 * - `brief` (NOT-08): the pre-meeting brief on an upcoming event's note.
 */
import type { ComponentType } from 'react';

import { EventBrief } from '@/features/briefs';
import type { CalendarEventId } from '@/features/calendar';
import type { MeetingId } from '@/features/meetings';

/** One suggested follow-up (NOT-07). */
export interface SuggestedFollowUp {
  /** Stable key. */
  id: string;
  /** What to do, in a few words ("Recap to Northwind"). */
  title: string;
  /** The meeting it follows up. */
  meetingId: MeetingId;
}

/** Props of the NOT-08 brief slot. */
export interface BriefSlotProps {
  eventId: CalendarEventId;
}

/** What other cards plug in; `null` until they ship. */
export interface HomeSlots {
  /** NOT-07: the follow-ups to suggest now. */
  useFollowUps: () => readonly SuggestedFollowUp[];
  /** NOT-08: the brief shown above an upcoming event's notes. */
  Brief: ComponentType<BriefSlotProps> | null;
}

/**
 * The slots as filled today: no follow-ups yet; NOT-08's `EventBrief`. The
 * event note itself (the meeting page of a note linked to an event) shows
 * NOT-08's `MeetingBrief` above the notepad and ORG-04's attendees in its
 * header.
 */
export const HOME_SLOTS: HomeSlots = {
  useFollowUps: () => [],
  Brief: EventBrief,
};
