/**
 * Pure helpers for the people and companies pages: initials and avatar
 * tones, dates as the lists say them, and searching and sorting rows.
 */
import { differenceInCalendarDays, format, isSameYear } from 'date-fns';

import type { EntitySort } from './types';

/** How many `--speaker-N` palette slots avatars cycle through. */
const PALETTE_SIZE = 6;

/** Up to two initials: first and last word (`Maya Chen` → `MC`). */
export function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const first = words[0]?.[0] ?? '';
  const last = words.length > 1 ? (words.at(-1)?.[0] ?? '') : '';
  return `${first}${last}`.toUpperCase();
}

/** A stable palette slot (1–6) for an ID, so a person keeps their colour everywhere. */
export function paletteSlot(id: string): number {
  let hash = 0;
  for (const char of id) hash = (hash * 31 + (char.codePointAt(0) ?? 0)) % 9973;
  return (hash % PALETTE_SIZE) + 1;
}

/** `Today`, `Yesterday`, `3 days ago`, `Sep 28`, or `Sep 28, 2025` in another year. */
export function relativeDay(iso: string, now: Date): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  const days = differenceInCalendarDays(now, date);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days > 1 && days < 7) return `${String(days)} days ago`;
  return format(date, isSameYear(date, now) ? 'MMM d' : 'MMM d, yyyy');
}

/** A due date as a list says it: `Today`, `Tomorrow` or `Oct 12`, and whether it has passed. */
export function dueLabel(iso: string, now: Date): { text: string; overdue: boolean } {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return { text: '', overdue: false };
  const days = differenceInCalendarDays(date, now);
  const overdue = days < 0;
  if (days === 0) return { text: 'Today', overdue };
  if (days === 1) return { text: 'Tomorrow', overdue };
  return { text: format(date, isSameYear(date, now) ? 'MMM d' : 'MMM d, yyyy'), overdue };
}

/** `Mon, Oct 12 · 15:00` for an upcoming event. */
export function eventTime(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : format(date, 'EEE, MMM d · HH:mm');
}

/** A recorded length: `44m`, `1h 23m`, or nothing when unknown. */
export function duration(seconds: number | null): string {
  if (seconds === null || seconds <= 0) return '';
  const minutes = Math.max(1, Math.round(seconds / 60));
  if (minutes < 60) return `${String(minutes)}m`;
  const rest = minutes % 60;
  return rest === 0
    ? `${String(minutes / 60)}h`
    : `${String(Math.floor(minutes / 60))}h ${String(rest)}m`;
}

/** A row the lists can search and sort. */
export interface Listable {
  name: string;
  lastMetAt: string | null;
  /** Further text the search matches (email, company, domain). */
  haystack: readonly (string | null)[];
}

/** True when every word of `query` starts a word of the row's name or fields. */
export function matchesQuery(row: Listable, query: string): boolean {
  const needles = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (needles.length === 0) return true;
  const words = [row.name, ...row.haystack]
    .filter((text): text is string => text !== null)
    .flatMap((text) => text.toLowerCase().split(/[\s@.]+/));
  return needles.every((needle) => words.some((word) => word.startsWith(needle)));
}

/** Most recently met first (never met last), or by name. */
export function compareRows(sort: EntitySort): (a: Listable, b: Listable) => number {
  const byName = (a: Listable, b: Listable) => a.name.localeCompare(b.name);
  if (sort === 'name') return byName;
  return (a, b) => {
    if (a.lastMetAt === b.lastMetAt) return byName(a, b);
    if (a.lastMetAt === null) return 1;
    if (b.lastMetAt === null) return -1;
    return b.lastMetAt.localeCompare(a.lastMetAt);
  };
}

/** `rows` matching `query`, in `sort` order. */
export function visibleRows<T extends Listable>(
  rows: readonly T[],
  query: string,
  sort: EntitySort,
): T[] {
  return rows.filter((row) => matchesQuery(row, query)).sort(compareRows(sort));
}
