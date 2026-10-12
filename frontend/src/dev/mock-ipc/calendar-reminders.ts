/**
 * ORG-02's second-half calendar commands for `pnpm dev:mock`: the reminder
 * window's event (the next event with a call link, so the window's page can
 * be opened on its own), dismiss, the open note, "Review last meeting" (a
 * fixture meeting before the event, same title first) and "Open in
 * Calendar" (a no-op).
 */
import type * as Generated from '@/lib/ipc/generated/calendar';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import { mockCalendarEvents as calendarEvents, fixtureMeetings } from './database';
import { latency } from './latency';

type ReminderCommands = Pick<
  typeof Generated.commands,
  'reminder' | 'dismissReminder' | 'setOpenNote' | 'lastMeeting' | 'openInCalendar'
>;

/** Whether the mock reminder window is still up. */
let reminding = true;
/** The event whose note the UI says is open. */
let openNote: Generated.CalendarEventId | null = null;

/** The event the open note belongs to, for tests and the console. */
export function mockOpenNote(): Generated.CalendarEventId | null {
  return openNote;
}

/** The next event with a call link: what the reminder window shows. */
function nextCall(): Generated.CalendarEvent | null {
  const now = Date.now();
  const ahead = calendarEvents
    .filter((event) => event.conferenceUrl !== null && Date.parse(event.endAt) > now)
    .sort((a, b) => Date.parse(a.startAt) - Date.parse(b.startAt));
  const [first] = ahead;
  return first === undefined ? null : structuredClone(first);
}

/** A fixture meeting before `event`: the same title first, else the latest. */
function lastBefore(event: Generated.CalendarEvent): Generated.RelatedMeeting | null {
  const before = fixtureMeetings
    .filter((meeting) => Date.parse(meeting.startedAt) < Date.parse(event.startAt))
    .sort((a, b) => Date.parse(b.startedAt) - Date.parse(a.startedAt));
  const sameTitle = before.find((meeting) => meeting.title === event.title);
  const found = sameTitle ?? before[0];
  if (found === undefined) return null;
  return {
    id: found.id as Generated.MeetingId,
    title: found.title,
    happenedAt: found.startedAt,
    by: sameTitle === undefined ? 'people' : 'titleAndPeople',
  };
}

function find<T>(
  id: Generated.CalendarEventId,
  then: (event: Generated.CalendarEvent) => T,
): CommandResult<T> {
  const event = calendarEvents.find((candidate) => candidate.id === id);
  if (event === undefined) {
    return failure<T>({ code: 'not_found', message: `calendar event ${id} was not found` });
  }
  return ok(then(event));
}

/** The reminder, open-note and Coming up menu commands. */
export const reminderCommands: ReminderCommands = {
  async reminder() {
    await latency();
    return ok(reminding ? nextCall() : null);
  },
  async dismissReminder() {
    await latency();
    reminding = false;
    return ok(null);
  },
  async setOpenNote(event) {
    await Promise.resolve();
    openNote = event;
    return ok(null);
  },
  async lastMeeting(id) {
    await latency();
    return find(id, lastBefore);
  },
  async openInCalendar(id) {
    await latency();
    return find(id, () => null);
  },
};
