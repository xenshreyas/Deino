/**
 * Due days in the user's local time. Rust stores a due day as noon local
 * time on that day (ADR 0028), so reading `dueAt` in the local zone always
 * gives the day back; everything here compares days, never instants.
 */
import {
  addDays,
  differenceInCalendarDays,
  format,
  isSameWeek,
  isValid,
  nextMonday,
  parseISO,
  startOfDay,
} from 'date-fns';

import { copy } from './copy';

/** The page's due groups, in order (ui.md §3 List row, sticky headers). */
export type DueGroup = 'overdue' | 'today' | 'this_week' | 'later' | 'none';

/** Every due group, in display order. */
export const DUE_GROUPS: readonly DueGroup[] = ['overdue', 'today', 'this_week', 'later', 'none'];

/** `YYYY-MM-DD` of a local day: what Rust's `NaiveDate` reads. */
export function toDay(date: Date): string {
  return format(date, 'yyyy-MM-dd');
}

/** The local day an ISO instant falls on, or `null` for an unreadable one. */
export function localDay(iso: string | null): Date | null {
  if (iso === null) return null;
  const date = parseISO(iso);
  return isValid(date) ? startOfDay(date) : null;
}

/** The local `YYYY-MM-DD` an ISO due instant falls on, or `null`. */
export function dueDay(iso: string | null): string | null {
  const day = localDay(iso);
  return day === null ? null : toDay(day);
}

/** Whole local days from `now`'s day to `iso`'s (negative when past). */
export function daysUntil(iso: string, now: Date): number {
  const day = localDay(iso);
  return day === null ? 0 : differenceInCalendarDays(day, startOfDay(now));
}

/** Which group a due date sits in, seen from `now`; weeks run Monday to Sunday. */
export function dueGroup(dueAt: string | null, now: Date): DueGroup {
  const day = localDay(dueAt);
  if (day === null) return 'none';
  const days = differenceInCalendarDays(day, startOfDay(now));
  if (days < 0) return 'overdue';
  if (days === 0) return 'today';
  return isSameWeek(day, now, { weekStartsOn: 1 }) ? 'this_week' : 'later';
}

/** The due chip's text: "Today", "Tomorrow", "Fri", "Oct 16" or "Oct 16, 2027". */
export function dueLabel(dueAt: string, now: Date): string {
  const day = localDay(dueAt);
  if (day === null) return '';
  const days = differenceInCalendarDays(day, startOfDay(now));
  if (days === 0) return copy.due.today;
  if (days === 1) return copy.due.tomorrow;
  if (days === -1) return copy.due.yesterday;
  if (days > 1 && days < 7) return format(day, 'EEE');
  return format(day, day.getFullYear() === now.getFullYear() ? 'MMM d' : 'MMM d, yyyy');
}

/** One quick choice in the due picker. */
export interface DueChoice {
  label: string;
  /** `YYYY-MM-DD`, or `null` to clear the due day. */
  day: string | null;
}

/** Today, tomorrow, next Monday and "No date", seen from `now`. */
export function quickDueChoices(now: Date): DueChoice[] {
  return [
    { label: copy.due.today, day: toDay(now) },
    { label: copy.due.tomorrow, day: toDay(addDays(now, 1)) },
    { label: copy.due.nextWeek, day: toDay(nextMonday(now)) },
    { label: copy.due.none, day: null },
  ];
}
