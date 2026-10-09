/**
 * Groups meetings under the list's sticky date headers (ui.md §3 List row):
 * Today, Yesterday, This week, then one bucket per month.
 */
import { format, isSameDay, isSameWeek, subDays } from 'date-fns';

import type { Meeting } from './types';

/** One date header and its meetings, newest first. */
export interface MeetingBucket {
  label: string;
  meetings: Meeting[];
}

/** The header a meeting created at `date` sits under, seen from `now`. */
export function bucketLabel(date: Date, now: Date): string {
  if (isSameDay(date, now)) return 'Today';
  if (isSameDay(date, subDays(now, 1))) return 'Yesterday';
  if (isSameWeek(date, now, { weekStartsOn: 1 }) && date < now) return 'This week';
  return format(date, date.getFullYear() === now.getFullYear() ? 'MMMM' : 'MMMM yyyy');
}

/** Buckets meetings (already newest first) in order; unreadable dates go to "Earlier". */
export function bucketMeetings(meetings: readonly Meeting[], now: Date): MeetingBucket[] {
  const buckets: MeetingBucket[] = [];
  for (const meeting of meetings) {
    const date = new Date(meeting.createdAt);
    const label = Number.isNaN(date.getTime()) ? 'Earlier' : bucketLabel(date, now);
    const last = buckets.at(-1);
    if (last?.label === label) last.meetings.push(meeting);
    else buckets.push({ label, meetings: [meeting] });
  }
  return buckets;
}

/** The row's trailing time: `14:00` today and yesterday, `Mon 14:00` this week, else `Oct 8`. */
export function meetingTime(iso: string, now: Date): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  if (isSameDay(date, now) || isSameDay(date, subDays(now, 1))) return format(date, 'HH:mm');
  if (isSameWeek(date, now, { weekStartsOn: 1 })) return format(date, 'EEE HH:mm');
  return format(date, 'MMM d');
}
