/**
 * How events and related meetings read, as pure functions of the data and
 * the time: "Today 10:00–10:30", "Oct 2", the guests who did not decline.
 */
import { addDays, format, isSameDay, isSameYear, subDays } from 'date-fns';

import { eventLinksCopy } from './copy';
import type { CalendarEvent } from './types';

/** "Today", "Yesterday", "Tomorrow", or `null` for any other day. */
function relativeDay(date: Date, now: Date): string | null {
  if (isSameDay(date, now)) return 'Today';
  if (isSameDay(date, subDays(now, 1))) return 'Yesterday';
  if (isSameDay(date, addDays(now, 1))) return 'Tomorrow';
  return null;
}

/** "Oct 2", or "Oct 2, 2025" in another year. */
function calendarDay(date: Date, now: Date): string {
  return format(date, isSameYear(date, now) ? 'MMM d' : 'MMM d, yyyy');
}

/** "Today", "Yesterday" or "Oct 2": when a related meeting took place. */
export function dayLabel(iso: string, now: Date): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  return relativeDay(date, now) ?? calendarDay(date, now);
}

/** "Today 10:00–10:30" or "Oct 7, 16:00–16:30": when an event runs. */
export function eventWhen(event: CalendarEvent, now: Date): string {
  const start = new Date(event.startAt);
  const end = new Date(event.endAt);
  const hours = `${format(start, 'HH:mm')}–${format(end, 'HH:mm')}`;
  const day = relativeDay(start, now);
  return day === null ? `${calendarDay(start, now)}, ${hours}` : `${day} ${hours}`;
}

/** The event's title, or "Untitled event". */
export function eventTitle(event: CalendarEvent): string {
  const title = event.title.trim();
  return title === '' ? eventLinksCopy.untitled : title;
}

/** The invitees who did not decline, by name (or address), organiser first. */
export function guestNames(event: CalendarEvent): string[] {
  return event.attendees
    .filter((guest) => guest.rsvp !== 'declined')
    .sort((a, b) => Number(b.isOrganizer) - Number(a.isOrganizer))
    .map((guest) => guest.name ?? guest.email ?? '')
    .filter((name) => name !== '');
}
