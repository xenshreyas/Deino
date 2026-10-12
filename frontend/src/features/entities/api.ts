/**
 * Typed calls to the `people` and `companies` plugins. Each resolves to the
 * data or throws an `IpcError`; components use `queries.ts` and
 * `mutations.ts` instead.
 */
import { convertFileSrc } from '@tauri-apps/api/core';

import { commands as companies } from '@/lib/ipc/generated/companies';
import { commands as people } from '@/lib/ipc/generated/people';
import { unwrap } from '@/lib/ipc/result';

import type {
  Company,
  CompanyFields,
  CompanyId,
  CompanyPage,
  CompanyRow,
  MeetingId,
  MeetingPerson,
  Person,
  PersonFields,
  PersonId,
  PersonPage,
  PersonRow,
  SummaryOutcome,
} from './types';

export const peopleApi = {
  /** Everyone the user meets, most recently met first. */
  list: (): Promise<PersonRow[]> => unwrap(people.list()),
  /** A person page; a merged person's is their survivor's. */
  page: (id: PersonId): Promise<PersonPage> => unwrap(people.page(id)),
  /** Up to eight people matching a name, word or email prefix. */
  search: (query: string): Promise<Person[]> => unwrap(people.search(query)),
  /** Adds a person; rejects with `invalid` or `conflict` (a taken address). */
  create: (fields: PersonFields): Promise<Person> => unwrap(people.create(fields)),
  /** Saves the person form. */
  update: (id: PersonId, fields: PersonFields): Promise<Person> =>
    unwrap(people.update(id, fields)),
  /** Saves the user's notes about a person. */
  setNotes: (id: PersonId, notes: string): Promise<null> => unwrap(people.setNotes(id, notes)),
  /** Merges `absorbed` into `survivor`, moving every link. */
  merge: (survivor: PersonId, absorbed: PersonId): Promise<Person> =>
    unwrap(people.merge(survivor, absorbed)),
  /** The person called `name`, added if nobody is. */
  fromMention: (name: string): Promise<Person> => unwrap(people.fromMention(name)),
  /** Rewrites a person's AI summary now. */
  refreshSummary: (id: PersonId): Promise<SummaryOutcome> => unwrap(people.refreshSummary(id)),
  /** A meeting's attendees, the organiser first. */
  attendees: (meetingId: MeetingId): Promise<MeetingPerson[]> =>
    unwrap(people.attendees(meetingId)),
  /** Adds someone to a meeting as an invitee. */
  addAttendee: (meetingId: MeetingId, personId: PersonId): Promise<null> =>
    unwrap(people.addAttendee(meetingId, personId)),
  /** Takes someone off a meeting. */
  removeAttendee: (meetingId: MeetingId, personId: PersonId): Promise<boolean> =>
    unwrap(people.removeAttendee(meetingId, personId)),
};

export const companiesApi = {
  /** Every company, most recently met first. */
  list: (): Promise<CompanyRow[]> => unwrap(companies.list()),
  /** A company page; a merged company's is its survivor's. */
  page: (id: CompanyId): Promise<CompanyPage> => unwrap(companies.page(id)),
  /** Adds a company; rejects with `invalid` or `conflict` (a taken domain). */
  create: (fields: CompanyFields): Promise<Company> => unwrap(companies.create(fields)),
  /** Saves the company form. */
  update: (id: CompanyId, fields: CompanyFields): Promise<Company> =>
    unwrap(companies.update(id, fields)),
  /** Saves the user's notes about a company. */
  setNotes: (id: CompanyId, notes: string): Promise<null> => unwrap(companies.setNotes(id, notes)),
  /** Merges `absorbed` into `survivor`, moving its people. */
  merge: (survivor: CompanyId, absorbed: CompanyId): Promise<Company> =>
    unwrap(companies.merge(survivor, absorbed)),
  /** Rewrites a company's AI summary now. */
  refreshSummary: (id: CompanyId): Promise<SummaryOutcome> => unwrap(companies.refreshSummary(id)),
  /** The cached logo, fetched once (or again with `refresh`); `permission_denied` while off. */
  fetchLogo: (id: CompanyId, refresh: boolean): Promise<string | null> =>
    unwrap(companies.fetchLogo(id, refresh)),
};

/** The URL a cached logo file shows at, through Tauri's asset protocol. */
export function logoUrl(path: string): string {
  return convertFileSrc(path);
}
