/**
 * The mock `action_items` and `action_item_views` tables, seeded from the
 * fixtures' `groundTruth.actionItems`: each expected item becomes an AI item
 * of its meeting, assigned by email, due at local noon on its day, cited at
 * the segment that holds its evidence quote. Items due before the fixture
 * week (before 1 October 2026) start done, so the page has history.
 */
import type {
  ActionItemEdit,
  ActionItemId,
  ActionItemView,
  CompanyId,
  CompanyOption,
  MeetingId,
  PersonId,
  PersonOption,
  SegmentId,
} from '@/lib/ipc/generated/action-items';
import type { Person } from '@/lib/ipc/generated/people';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

/** The fixtures' user ("Me"). */
export const USER_EMAIL = 'shreyas@deino.dev';
/** Fixture items due before this day start done. */
const DONE_BEFORE = '2026-10-01';

/** A due day as the app stores it: noon, local time. */
export function localNoon(day: string): string {
  return new Date(`${day}T12:00:00`).toISOString();
}

function domainOf(email: string | null): string | null {
  return email?.split('@')[1] ?? null;
}

/** The fixtures' companies, with the seeder's IDs. */
export function fixtureCompanies(fixtures: readonly FixtureMeeting[]): CompanyOption[] {
  const byDomain = new Map<string, CompanyOption>();
  for (const fixture of fixtures) {
    for (const company of fixture.companies) {
      byDomain.set(company.domain, {
        id: fixtureUuid(`company/${company.domain}`) as CompanyId,
        name: company.name,
      });
    }
  }
  return [...byDomain.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** People as the filter bar lists them: the user first, then by name. */
export function personOptions(people: readonly Person[]): PersonOption[] {
  const options = people.map((person) => ({
    id: person.id,
    name: person.name,
    email: person.email,
    isMe: person.email === USER_EMAIL,
  }));
  return [...options.filter((person) => person.isMe), ...options.filter((person) => !person.isMe)];
}

/** One fixture's expected items as stored rows. */
function rowsOf(fixture: FixtureMeeting, people: readonly Person[]): ActionItemView[] {
  return fixture.groundTruth.actionItems.map((expected, index) => {
    const person = people.find((candidate) => candidate.email === expected.assignee) ?? null;
    const line = fixture.segments.findIndex((segment) => segment.text.includes(expected.evidence));
    const done = expected.due !== null && expected.due < DONE_BEFORE;
    const savedAt = new Date(Date.parse(fixture.startedAt) + fixture.durationMs).toISOString();
    return {
      id: fixtureUuid(`action-item/${fixture.id}/${String(index)}`) as ActionItemId,
      meetingId: fixture.id as MeetingId,
      title: expected.title,
      description: '',
      assigneePersonId: person?.id ?? null,
      dueAt: expected.due === null ? null : localNoon(expected.due),
      status: done ? 'done' : 'open',
      priority: null,
      sourceSegmentId:
        line === -1 ? null : (fixtureUuid(`segment/${fixture.id}/${String(line)}`) as SegmentId),
      completedAt: done ? savedAt : null,
      createdBy: 'ai',
      createdAt: savedAt,
      updatedAt: savedAt,
      meetingTitle: fixture.title,
      meetingHeldAt: fixture.startedAt,
      assigneeName: person?.name ?? null,
      assigneeIsMe: person?.email === USER_EMAIL,
      sourceStart: line === -1 ? null : (fixture.segments[line]?.startMs ?? null),
    };
  });
}

/** Every fixture's expected items. */
export function fixtureActionItems(
  fixtures: readonly FixtureMeeting[],
  people: readonly Person[],
): ActionItemView[] {
  return fixtures.flatMap((fixture) => rowsOf(fixture, people));
}

/** The companies a row is linked to: its assignee's and its meeting's attendees'. */
export function rowCompanies(
  item: ActionItemView,
  fixtures: readonly FixtureMeeting[],
  people: readonly Person[],
): CompanyId[] {
  const assignee = people.find((person) => person.id === item.assigneePersonId);
  const attendees = fixtures.find((fixture) => fixture.id === item.meetingId)?.attendees ?? [];
  const domains = [
    domainOf(assignee?.email ?? null),
    ...attendees.map((attendee) => domainOf(attendee.email)),
  ];
  return domains
    .filter((domain) => domain !== null)
    .map((domain) => fixtureUuid(`company/${domain}`) as CompanyId);
}

/** `item` after `edit`, as Rust applies it (noon for days, names for people). */
export function edited(
  item: ActionItemView,
  edit: ActionItemEdit,
  context: { now: Date; people: readonly PersonOption[] },
): ActionItemView {
  const touched = { ...item, updatedAt: context.now.toISOString() };
  switch (edit.field) {
    case 'title':
      return { ...touched, title: edit.title.trim() };
    case 'description':
      return { ...touched, description: edit.description.trim() };
    case 'priority':
      return { ...touched, priority: edit.priority };
    case 'due':
      return { ...touched, dueAt: edit.date === null ? null : localNoon(edit.date) };
    case 'assignee':
      return assigned(touched, edit.personId, context.people);
    case 'status':
      return {
        ...touched,
        status: edit.status,
        completedAt:
          edit.status === 'done' ? (item.completedAt ?? context.now.toISOString()) : null,
      };
  }
}

function assigned(item: ActionItemView, id: PersonId | null, people: readonly PersonOption[]) {
  const person = people.find((candidate) => candidate.id === id) ?? null;
  return {
    ...item,
    assigneePersonId: person?.id ?? null,
    assigneeName: person?.name ?? null,
    assigneeIsMe: person?.isMe ?? false,
  };
}
