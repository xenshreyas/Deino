/**
 * Pure helpers behind Coming up's row actions: the invitees' `mailto:`
 * link and the record toggle's next choice.
 */
import type { CalendarEvent, RecordChoice, RecordDecision } from '@/features/calendar';

/** A `mailto:` link to every invitee with an address, or `null` when none has one. */
export function inviteesMailto(event: CalendarEvent): string | null {
  const emails = event.attendees
    .map((attendee) => attendee.email)
    .filter((email): email is string => email !== null && email !== '');
  if (emails.length === 0) return null;
  const subject = encodeURIComponent(event.title);
  return `mailto:${emails.map(encodeURIComponent).join(',')}?subject=${subject}`;
}

/** True when the event repeats, so the toggle asks "this one or all". */
export function isRecurring(event: CalendarEvent): boolean {
  return event.seriesId !== null;
}

/** Flipping the toggle: record what does not record now, and the reverse. */
export function flipChoice(decision: RecordDecision, scope: RecordChoice['scope']): RecordChoice {
  return { record: !decision.record, scope };
}
