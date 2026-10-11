/**
 * The mock `people`, `companies` and `meeting_attendees` tables behind the
 * People and Companies pages, seeded from the fixtures like the Rust seeder
 * (same `fixtureUuid` IDs): every attendee is a person at the company of
 * their email domain, attends their fixture meetings, and the user is "Me".
 * A few people and companies carry a briefing, so the pages show one.
 * Writes follow the Rust rules that matter for the UI (merges hide the
 * absorbed entry and move its attendance) and report their tables.
 */
import type { Company, CompanyId } from '@/lib/ipc/generated/companies';
import type { AttendeeRole, MeetingId, Person, PersonId } from '@/lib/ipc/generated/people';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

/** The fixtures' user. */
const USER_EMAIL = 'shreyas@deino.dev';

/** One attendance row. */
export interface Attendance {
  meetingId: MeetingId;
  personId: PersonId;
  role: AttendeeRole;
}

/** Briefings the mock shows, keyed by email or domain. Fictional, like the fixtures. */
const BRIEFINGS: Readonly<Record<string, string>> = {
  'maya@deino.dev':
    'Senior backend engineer who owns the webhook queue and the sync rewrite. In the last 1:1 she agreed to split the queue by tenant before the v1.5 release and asked for a second reviewer on the migration.',
  'marcus.webb@brightkite-logistics.com':
    'VP Operations at Brightkite, evaluating Deino for 40 dispatchers. He cares about routing accuracy and SSO; a one-region pilot is on the table once security review passes.',
  'brightkite-logistics.com':
    'Logistics prospect in discovery. Marcus Webb sponsors a one-region pilot; Hannah Pike leads the security review, which blocks the next step.',
};

/** A new company row for `domain`. */
function companyRow(name: string, domain: string, at: string): Company {
  return {
    id: fixtureUuid(`company/${domain}`) as CompanyId,
    name,
    domain,
    logoPath: null,
    notesMd: '',
    summaryMd: BRIEFINGS[domain] ?? null,
    summaryUpdatedAt: BRIEFINGS[domain] === undefined ? null : at,
    logoCheckedAt: null,
    mergedInto: null,
    createdAt: at,
    updatedAt: at,
  };
}

/** The fixture companies, once per domain. */
function fixtureCompanies(fixtures: readonly FixtureMeeting[]): Company[] {
  const byDomain = new Map<string, Company>();
  for (const fixture of fixtures) {
    for (const company of fixture.companies) {
      byDomain.set(company.domain, companyRow(company.name, company.domain, fixture.startedAt));
    }
  }
  return [...byDomain.values()];
}

/** `people` at their companies, the user marked, briefings attached. */
function linkedPeople(people: readonly Person[], companies: readonly Company[]): Person[] {
  return people.map((person) => {
    const domain = person.email?.split('@')[1] ?? null;
    const company = companies.find((candidate) => candidate.domain === domain) ?? null;
    const briefing = person.email === null ? undefined : BRIEFINGS[person.email];
    return {
      ...person,
      companyId: company?.id ?? null,
      isMe: person.email === USER_EMAIL,
      summaryMd: briefing ?? null,
      summaryUpdatedAt: briefing === undefined ? null : person.updatedAt,
    };
  });
}

/** Who attended which fixture meeting. */
function fixtureAttendance(fixtures: readonly FixtureMeeting[]): Attendance[] {
  return fixtures.flatMap((fixture) =>
    fixture.attendees.map((attendee) => ({
      meetingId: fixture.id as MeetingId,
      personId: fixtureUuid(`person/${attendee.email}`) as PersonId,
      role: attendee.role,
    })),
  );
}

/** The mutable entity tables for one dev:mock session. */
export class MockEntities {
  people: Person[];
  companies: Company[];
  attendance: Attendance[];

  constructor(fixtures: readonly FixtureMeeting[], people: readonly Person[]) {
    this.companies = fixtureCompanies(fixtures);
    this.people = linkedPeople(people, this.companies);
    this.attendance = fixtureAttendance(fixtures);
  }

  /** The person with `id`, or whom they were merged into. */
  person(id: PersonId): Person | undefined {
    const found = this.people.find((person) => person.id === id);
    if (found?.mergedInto == null) return found;
    return this.people.find((person) => person.id === found.mergedInto);
  }

  /** The company with `id`, or whichever it was merged into. */
  company(id: CompanyId): Company | undefined {
    const found = this.companies.find((company) => company.id === id);
    if (found?.mergedInto == null) return found;
    return this.companies.find((company) => company.id === found.mergedInto);
  }

  /** The meetings `person` attended. */
  meetingsOf(person: PersonId): MeetingId[] {
    return this.attendance.filter((row) => row.personId === person).map((row) => row.meetingId);
  }

  /** Moves `absorbed`'s attendance to `survivor` and hides `absorbed`. */
  mergePeople(survivor: Person, absorbed: Person, now: string): void {
    const taken = new Set(this.meetingsOf(survivor.id));
    this.attendance = this.attendance
      .filter((row) => !(row.personId === absorbed.id && taken.has(row.meetingId)))
      .map((row) => (row.personId === absorbed.id ? { ...row, personId: survivor.id } : row));
    this.people = this.people.map((person) => {
      if (person.id === absorbed.id) return { ...person, mergedInto: survivor.id, updatedAt: now };
      if (person.id !== survivor.id) return person;
      return { ...person, email: person.email ?? absorbed.email, updatedAt: now };
    });
  }

  /** Moves `absorbed`'s people to `survivor` and hides `absorbed`. */
  mergeCompanies(survivor: Company, absorbed: Company, now: string): void {
    this.people = this.people.map((person) =>
      person.companyId === absorbed.id ? { ...person, companyId: survivor.id } : person,
    );
    this.companies = this.companies.map((company) =>
      company.id === absorbed.id
        ? { ...company, mergedInto: survivor.id, updatedAt: now }
        : company,
    );
  }
}
