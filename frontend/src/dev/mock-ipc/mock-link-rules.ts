/**
 * ORG-03's linking rules for the mock, as in `deino_core::calendar::linking`:
 * the nearest event to a recording start, the picker's suggestions, and
 * which meetings are related.
 */
import type * as Calendar from '@/lib/ipc/generated/calendar';

type CalendarEvent = Calendar.CalendarEvent;

const MINUTE_MS = 60_000;
/** `LINK_WINDOW`: an event starting this soon is the one a recording belongs to. */
const LINK_WINDOW_MS = 15 * MINUTE_MS;
/** `SUGGEST_BEFORE` and `SUGGEST_AFTER`: the picker's span around a meeting. */
const SUGGEST_BEFORE_MS = 24 * 60 * MINUTE_MS;
const SUGGEST_AFTER_MS = 60 * MINUTE_MS;
/** `MAX_SUGGESTIONS` and `RELATED_LIMIT`. */
const MAX_SUGGESTIONS = 20;
export const RELATED_LIMIT = 5;

/** Timed, and not declined. */
function isLinkable(event: CalendarEvent): boolean {
  return !event.isAllDay && event.myRsvp !== 'declined';
}

/** Milliseconds between the event's start and `at`. */
function distance(event: CalendarEvent, at: number): number {
  return Math.abs(Date.parse(event.startAt) - at);
}

/** The event a recording starting at `now` belongs to (`nearest_event`). */
export function nearestEvent(events: readonly CalendarEvent[], now: number): CalendarEvent | null {
  const near = events.filter(
    (event) =>
      isLinkable(event) &&
      Date.parse(event.startAt) - LINK_WINDOW_MS <= now &&
      now < Date.parse(event.endAt),
  );
  const sorted = near.sort(
    (a, b) =>
      distance(a, now) - distance(b, now) ||
      Number(Date.parse(a.startAt) > now) - Number(Date.parse(b.startAt) > now),
  );
  return sorted[0] ?? null;
}

/** The picker's suggestions around `anchor` (`suggestion_range` + `rank_suggestions`). */
export function suggestionsAround(
  events: readonly CalendarEvent[],
  anchor: number,
): CalendarEvent[] {
  const [start, end] = [anchor - SUGGEST_BEFORE_MS, anchor + SUGGEST_AFTER_MS];
  return events
    .filter((event) => isLinkable(event))
    .filter((event) => Date.parse(event.startAt) < end && Date.parse(event.endAt) > start)
    .sort((a, b) => distance(a, anchor) - distance(b, anchor) || b.startAt.localeCompare(a.startAt))
    .slice(0, MAX_SUGGESTIONS);
}

/** What a meeting is related by. */
export interface RelationSubject {
  id: string;
  title: string;
  seriesId: string | null;
  happenedAt: string;
  /** Attendee emails, the user left out. */
  people: ReadonlySet<string>;
}

/** True when both sets hold the same members. */
export function sameSet(a: ReadonlySet<string>, b: ReadonlySet<string>): boolean {
  return a.size === b.size && [...a].every((member) => b.has(member));
}

/** Why `candidate` is related to `subject`, if it is (`related::relation`). */
function relation(subject: RelationSubject, candidate: RelationSubject): Calendar.RelatedBy | null {
  if (subject.seriesId !== null && candidate.seriesId === subject.seriesId) return 'series';
  const samePeople = subject.people.size > 0 && sameSet(subject.people, candidate.people);
  return candidate.title === subject.title && samePeople ? 'titleAndPeople' : null;
}

/** The related meetings with "Previous in series" (`related::relate`). */
export function relate(
  subject: RelationSubject,
  candidates: readonly RelationSubject[],
): Calendar.RelatedMeetings {
  const related = candidates
    .filter((candidate) => candidate.id !== subject.id)
    .flatMap((candidate) => {
      const by = relation(subject, candidate);
      if (by === null) return [];
      const { id, title, happenedAt } = candidate;
      return [{ id: id as Calendar.MeetingId, title, happenedAt, by }];
    })
    .sort((a, b) => b.happenedAt.localeCompare(a.happenedAt));
  const index = related.findIndex((meeting) => meeting.happenedAt < subject.happenedAt);
  const previous = index === -1 ? null : (related.splice(index, 1)[0] ?? null);
  return { previous, others: related.slice(0, RELATED_LIMIT) };
}
