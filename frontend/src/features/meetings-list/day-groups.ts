/**
 * Groups the list's rows under one sticky header per local day (ui.md §3
 * List row): "Today", "Yesterday", then the date. Rows arrive newest first,
 * so a group never splits across pages.
 */
import { format, isSameDay, isSameYear, subDays } from 'date-fns';

import { copy } from './copy';
import type { ListedMeeting } from './types';

/** One day's header and its meetings, newest first. */
export interface DayGroup {
  /** The day, `yyyy-MM-dd` in local time. */
  key: string;
  label: string;
  meetings: ListedMeeting[];
}

/** The header of `date`'s day, seen from `now`. */
export function dayLabel(date: Date, now: Date): string {
  if (isSameDay(date, now)) return copy.today;
  if (isSameDay(date, subDays(now, 1))) return copy.yesterday;
  return format(date, isSameYear(date, now) ? 'EEEE, MMMM d' : 'EEEE, MMMM d, yyyy');
}

/** Groups `meetings` (newest first) by the local day they were saved. */
export function groupByDay(meetings: readonly ListedMeeting[], now: Date): DayGroup[] {
  const groups: DayGroup[] = [];
  for (const meeting of meetings) {
    const date = new Date(meeting.createdAt);
    const key = format(date, 'yyyy-MM-dd');
    const last = groups.at(-1);
    if (last?.key === key) last.meetings.push(meeting);
    else groups.push({ key, label: dayLabel(date, now), meetings: [meeting] });
  }
  return groups;
}

/** A row's trailing time: `14:05`. */
export function rowTime(iso: string): string {
  return format(new Date(iso), 'HH:mm');
}

/** A recorded length in seconds as `45m` or `1h 5m`; empty when unknown or under a minute. */
export function rowDuration(seconds: number | null): string {
  if (seconds === null || seconds < 60) return '';
  return copy.duration(Math.round(seconds / 60));
}
