/**
 * Pure helpers of search: moment times, dates and the `@` being typed.
 */
import { format } from 'date-fns';

const MS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;

/** A moment's place in the recording: `12:04`, or `1:02:03` past an hour. */
export function momentTime(ms: number): string {
  const whole = Math.max(0, Math.floor(ms / MS_PER_SECOND));
  const hours = Math.floor(whole / SECONDS_PER_HOUR);
  const minutes = Math.floor((whole % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
  const seconds = String(whole % SECONDS_PER_MINUTE).padStart(2, '0');
  if (hours === 0) return `${minutes}:${seconds}`;
  return `${hours}:${String(minutes).padStart(2, '0')}:${seconds}`;
}

/** `Oct 8`, or an empty string for a missing or unreadable date. */
export function shortDate(iso: string | null): string {
  if (iso === null) return '';
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? '' : format(date, 'MMM d');
}

/** The `@` mention being typed at the end of `text`, if any. */
export interface ActiveMention {
  /** What follows the `@` (may be empty: the picker shows everything). */
  prefix: string;
  /** `text` without the mention, for the search itself. */
  rest: string;
}

/** An `@word` (or a lone `@`) the user is typing at the end of `text`. */
export function activeMention(text: string): ActiveMention | null {
  const match = /(^|\s)@([^\s@"]*)$/u.exec(text);
  if (match === null) return null;
  const [whole, lead = '', prefix = ''] = match;
  return { prefix, rest: text.slice(0, text.length - whole.length + lead.length) };
}
