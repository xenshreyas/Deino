/**
 * "Coming up" as a pure function of the events and the time: the range to
 * ask Rust for, the day groups, and each row's time and countdown.
 */
import {
  addDays,
  differenceInMinutes,
  format,
  isSameDay,
  max as latest,
  startOfDay,
} from 'date-fns';

import { calendarCopy } from './copy';
import type { CalendarEvent, TimeRange } from './types';

/** How many days ahead "Coming up" lists; calendar sync reads 14 ahead. */
export const COMING_UP_DAYS = 14;

/** Within this many minutes of its start, a row shows a countdown. */
const COUNTDOWN_MINUTES = 60;

/**
 * The range "Coming up" asks for: from the start of today to the end of
 * the 14th day ahead. It only changes at midnight, so the query key stays
 * put while the list's clock ticks.
 */
export function comingUpRange(now: Date): TimeRange {
  const today = startOfDay(now);
  return {
    start: today.toISOString(),
    end: addDays(today, COMING_UP_DAYS + 1).toISOString(),
  };
}

/** One day of the agenda. */
export interface AgendaDay {
  /** The day, `yyyy-MM-dd`, for keys. */
  key: string;
  /** "Today", "Tomorrow" or "Mon, Oct 12". */
  label: string;
  events: CalendarEvent[];
}

/**
 * The events not over at `now`, in order, grouped by the day they start;
 * one that started earlier and still runs sits under today.
 */
export function agendaDays(events: readonly CalendarEvent[], now: Date): AgendaDay[] {
  const days: AgendaDay[] = [];
  for (const event of events) {
    if (new Date(event.endAt) <= now) continue;
    const day = latest([startOfDay(new Date(event.startAt)), startOfDay(now)]);
    const key = format(day, 'yyyy-MM-dd');
    const last = days.at(-1);
    if (last?.key === key) last.events.push(event);
    else days.push({ key, label: dayLabel(day, now), events: [event] });
  }
  return days;
}

/** "Today", "Tomorrow", or the weekday and date. */
export function dayLabel(day: Date, now: Date): string {
  if (isSameDay(day, now)) return calendarCopy.today;
  if (isSameDay(day, addDays(now, 1))) return calendarCopy.tomorrow;
  return format(day, 'EEE, MMM d');
}

/** The row's time: "All day", or the local start `HH:mm`. */
export function eventTime(event: CalendarEvent): string {
  return event.isAllDay ? calendarCopy.allDay : format(new Date(event.startAt), 'HH:mm');
}

/** "Now" while it runs, "in 12m" in the hour before it starts, else `null`. */
export function startsIn(event: CalendarEvent, now: Date): string | null {
  if (event.isAllDay) return null;
  const start = new Date(event.startAt);
  if (start <= now) return calendarCopy.now;
  const minutes = differenceInMinutes(start, now, { roundingMethod: 'ceil' });
  return minutes <= COUNTDOWN_MINUTES ? calendarCopy.inMinutes(minutes) : null;
}
