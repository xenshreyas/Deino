/**
 * What the People and Companies commands return in dev:mock, read from the
 * mock entity tables with the Rust rules: lists exclude merged entries and
 * the user, count each meeting once, and order by last met; pages carry
 * meetings (most recent first), open action items and upcoming events.
 */
import type { CompanyPage, CompanyRow } from '@/lib/ipc/generated/companies';
import type {
  EntityActionItem,
  EntityMeeting,
  MeetingId,
  MeetingPerson,
  Person,
  PersonPage,
  PersonRow,
  UpcomingEvent,
} from '@/lib/ipc/generated/people';

import { fixtureCalendarEvents } from './calendar-fixtures';
import type { FixtureMeeting } from './fixture-schema';
import { fixtureActionItems } from './mock-action-items';
import type { MockEntities } from './mock-entities';

/** `deino_core::entities::UPCOMING_DAYS` and `MAX_UPCOMING`. */
const UPCOMING_MS = 14 * 24 * 60 * 60 * 1000;
const MAX_UPCOMING = 5;

/** Reads pages from `entities` over the fixture meetings. */
export class MockEntityPages {
  private readonly meetings: Map<string, EntityMeeting>;

  constructor(
    private readonly entities: MockEntities,
    private readonly fixtures: readonly FixtureMeeting[],
  ) {
    this.meetings = new Map(
      fixtures.map((fixture) => [
        fixture.id,
        {
          id: fixture.id as MeetingId,
          title: fixture.title,
          at: fixture.startedAt,
          durationS: Math.round(fixture.durationMs / 1000),
        },
      ]),
    );
  }

  /** The meetings of `people`, each once, most recent first. */
  meetingsOf(people: readonly Person[]): EntityMeeting[] {
    const ids = new Set(people.flatMap((person) => this.entities.meetingsOf(person.id)));
    return [...ids]
      .map((id) => this.meetings.get(id))
      .filter((meeting) => meeting !== undefined)
      .sort((a, b) => b.at.localeCompare(a.at));
  }

  /** A list row for `person`. */
  personRow(person: Person): PersonRow {
    const company = person.companyId === null ? undefined : this.entities.company(person.companyId);
    const meetings = this.meetingsOf([person]);
    return {
      id: person.id,
      name: person.name,
      email: person.email,
      title: person.title,
      companyId: person.companyId,
      companyName: company?.name ?? null,
      companyLogoPath: company?.logoPath ?? null,
      lastMetAt: meetings[0]?.at ?? null,
      meetingCount: meetings.length,
    };
  }

  /** Listed people, optionally only those at one company. */
  listed(companyId: string | null = null): Person[] {
    return this.entities.people.filter(
      (person) =>
        person.mergedInto === null &&
        !person.isMe &&
        (companyId === null || person.companyId === companyId),
    );
  }

  /** The People list. */
  personRows(): PersonRow[] {
    return this.listed()
      .map((person) => this.personRow(person))
      .sort(byLastMet);
  }

  /** The Companies list. */
  companyRows(): CompanyRow[] {
    return this.entities.companies
      .filter((company) => company.mergedInto === null)
      .map((company) => {
        const people = this.listed(company.id);
        const meetings = this.meetingsOf(people);
        return {
          id: company.id,
          name: company.name,
          domain: company.domain,
          logoPath: company.logoPath,
          peopleCount: people.length,
          meetingCount: meetings.length,
          lastMetAt: meetings[0]?.at ?? null,
        };
      })
      .sort(byLastMet);
  }

  /** The open items assigned to any of `people`. */
  openItems(people: readonly Person[]): EntityActionItem[] {
    const ids = new Set(people.map((person) => person.id));
    return fixtureActionItems(this.fixtures, this.entities.people)
      .filter(
        (item) =>
          item.status === 'open' &&
          item.assigneePersonId !== null &&
          ids.has(item.assigneePersonId),
      )
      .map(({ id, title, dueAt, assigneePersonId, meetingId }) => ({
        id,
        title,
        dueAt,
        assigneePersonId,
        meetingId,
      }))
      .sort(byDue);
  }

  /** Events in the next two weeks inviting someone `wanted` accepts. */
  upcoming(wanted: (email: string) => boolean, now = new Date()): UpcomingEvent[] {
    const until = now.getTime() + UPCOMING_MS;
    return fixtureCalendarEvents(this.fixtures, now)
      .filter(
        (event) => Date.parse(event.startAt) >= now.getTime() && Date.parse(event.startAt) < until,
      )
      .filter((event) =>
        event.attendees.some((attendee) => attendee.email !== null && wanted(attendee.email)),
      )
      .sort((a, b) => a.startAt.localeCompare(b.startAt))
      .slice(0, MAX_UPCOMING)
      .map(({ id, title, startAt, endAt, conferenceUrl }) => ({
        id,
        title,
        startAt,
        endAt,
        conferenceUrl,
      }));
  }

  /** A person page. */
  personPage(person: Person): PersonPage {
    const company =
      person.companyId === null ? null : (this.entities.company(person.companyId) ?? null);
    return {
      person,
      company,
      meetings: this.meetingsOf([person]),
      actionItems: this.openItems([person]),
      upcoming: this.upcoming((email) => email === person.email),
    };
  }

  /** A company page (`company` must be listed). */
  companyPage(companyId: string): CompanyPage | null {
    const company = this.entities.companies.find((candidate) => candidate.id === companyId);
    if (company === undefined) return null;
    const people = this.listed(company.id);
    return {
      company,
      people: people.map((person) => this.personRow(person)).sort(byLastMet),
      meetings: this.meetingsOf(people),
      actionItems: this.openItems(people),
      upcoming: this.upcoming((email) => email.endsWith(`@${company.domain ?? ''}`)),
    };
  }

  /** A meeting's attendees as the header shows them: organiser first, the user last. */
  attendees(meetingId: MeetingId): MeetingPerson[] {
    return this.entities.attendance
      .filter((row) => row.meetingId === meetingId)
      .flatMap((row) => {
        const person = this.entities.person(row.personId);
        if (person === undefined) return [];
        const company =
          person.companyId === null ? undefined : this.entities.company(person.companyId);
        return [
          {
            personId: person.id,
            name: person.name,
            email: person.email,
            title: person.title,
            companyId: person.companyId,
            companyLogoPath: company?.logoPath ?? null,
            role: row.role,
            isMe: person.isMe,
          },
        ];
      })
      .sort(
        (a, b) =>
          Number(a.isMe) - Number(b.isMe) ||
          Number(b.role === 'organizer') - Number(a.role === 'organizer'),
      );
  }
}

/** Soonest due first, undated last, like `ActionItemRepo::list_for_assignee`. */
function byDue(a: EntityActionItem, b: EntityActionItem): number {
  if (a.dueAt === b.dueAt) return a.id.localeCompare(b.id);
  if (a.dueAt === null) return 1;
  if (b.dueAt === null) return -1;
  return a.dueAt.localeCompare(b.dueAt);
}

/** Most recently met first, never met last, then by name. */
function byLastMet<T extends { name: string; lastMetAt: string | null }>(a: T, b: T): number {
  if (a.lastMetAt !== b.lastMetAt) {
    if (a.lastMetAt === null) return 1;
    if (b.lastMetAt === null) return -1;
    return b.lastMetAt.localeCompare(a.lastMetAt);
  }
  return a.name.localeCompare(b.name);
}
