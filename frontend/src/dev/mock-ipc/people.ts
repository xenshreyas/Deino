/**
 * Mock of `@/lib/ipc/generated/people`: @mention search, the People list and
 * pages, the person form, notes, merges and a meeting's attendees, over the
 * mock entity tables (`mock-entities.ts`). Writes report their tables, so
 * `DbChanged` refreshes the pages as in the app. Summaries answer
 * `no_model`: dev:mock has no LLM.
 */
import type * as Generated from '@/lib/ipc/generated/people';

import { mockDatabase, mockEntities, mockEntityPages } from './database';
import { latency } from './latency';
import { failure, ok } from './mock-database';
import { searchPeople } from './mock-people';

type Person = Generated.Person;
type PersonId = Generated.PersonId;

let created = 0;

const notFound = (id: string) =>
  failure<never>({ code: 'not_found', message: `person ${id} was not found` });
const invalid = (message: string) => failure<never>({ code: 'invalid', message });

/** The form's fields as Rust reads them (trimmed, blank is none), or why not. */
function checked(fields: Generated.PersonFields, self: PersonId | null) {
  const name = fields.name.trim();
  const email = fields.email?.trim().toLowerCase() || null;
  if (name === '') return invalid('a name cannot be empty');
  if (email !== null && !/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email)) {
    return invalid('that is not an email address');
  }
  const taken = mockEntities.people.some((person) => person.email === email && person.id !== self);
  if (email !== null && taken) {
    return failure<never>({
      code: 'conflict',
      message: 'someone else already has that email address; merge the two instead',
    });
  }
  return ok({ name, email, title: fields.title?.trim() || null, companyId: fields.companyId });
}

/** A new person from checked fields. */
function newPerson(fields: Pick<Person, 'name' | 'email' | 'title' | 'companyId'>): Person {
  const now = new Date().toISOString();
  created += 1;
  return {
    id: `0199d0a0-0000-7000-8000-${String(created).padStart(12, '0')}` as PersonId,
    ...fields,
    avatarPath: null,
    notesMd: '',
    summaryMd: null,
    summaryUpdatedAt: null,
    isMe: false,
    mergedInto: null,
    createdAt: now,
    updatedAt: now,
  };
}

/** Replaces `person` and reports the write. */
function save(person: Person): Person {
  mockEntities.people = mockEntities.people.map((row) => (row.id === person.id ? person : row));
  mockDatabase.touch(['people']);
  return person;
}

export const commands = {
  async search(query) {
    await latency();
    return ok(
      searchPeople(
        mockEntities.people.filter((person) => person.mergedInto === null),
        query,
      ),
    );
  },
  async list() {
    await latency();
    return ok(mockEntityPages.personRows());
  },
  async page(id) {
    await latency();
    const person = mockEntities.person(id);
    return person === undefined ? notFound(id) : ok(mockEntityPages.personPage(person));
  },
  async create(fields) {
    await latency();
    const result = checked(fields, null);
    if (result.status === 'error') return result;
    const person = newPerson(result.data);
    mockEntities.people = [...mockEntities.people, person];
    mockDatabase.touch(['people']);
    return ok(person);
  },
  async update(id, fields) {
    await latency();
    const person = mockEntities.person(id);
    const result = checked(fields, id);
    if (person === undefined) return notFound(id);
    if (result.status === 'error') return result;
    return ok(save({ ...person, ...result.data, updatedAt: new Date().toISOString() }));
  },
  async setNotes(id, notes) {
    await latency();
    const person = mockEntities.person(id);
    if (person === undefined) return notFound(id);
    save({ ...person, notesMd: notes });
    return ok(null);
  },
  async merge(survivor, absorbed) {
    await latency();
    if (survivor === absorbed) return invalid('pick two different entries to merge');
    const kept = mockEntities.person(survivor);
    const gone = mockEntities.person(absorbed);
    if (kept === undefined) return notFound(survivor);
    if (gone === undefined) return notFound(absorbed);
    mockEntities.mergePeople(kept, gone, new Date().toISOString());
    mockDatabase.touch(['people', 'meeting_attendees']);
    return ok(mockEntities.person(survivor) ?? kept);
  },
  async fromMention(name) {
    await latency();
    const wanted = name.trim();
    if (wanted === '') return invalid('a name cannot be empty');
    const existing = mockEntities.people.find(
      (person) => person.mergedInto === null && person.name.toLowerCase() === wanted.toLowerCase(),
    );
    if (existing !== undefined) return ok(existing);
    const person = newPerson({ name: wanted, email: null, title: null, companyId: null });
    mockEntities.people = [...mockEntities.people, person];
    mockDatabase.touch(['people']);
    return ok(person);
  },
  async refreshSummary(id) {
    await latency();
    return mockEntities.person(id) === undefined ? notFound(id) : ok('no_model' as const);
  },
  async attendees(meetingId) {
    await latency();
    return ok(mockEntityPages.attendees(meetingId));
  },
  async addAttendee(meetingId, personId) {
    await latency();
    const person = mockEntities.person(personId);
    if (person === undefined) return notFound(personId);
    if (!mockEntities.meetingsOf(person.id).includes(meetingId)) {
      mockEntities.attendance = [
        ...mockEntities.attendance,
        { meetingId, personId: person.id, role: 'invitee' },
      ];
      mockDatabase.touch(['meeting_attendees']);
    }
    return ok(null);
  },
  async removeAttendee(meetingId, personId) {
    await latency();
    const before = mockEntities.attendance.length;
    mockEntities.attendance = mockEntities.attendance.filter(
      (row) => !(row.meetingId === meetingId && row.personId === personId),
    );
    mockDatabase.touch(['meeting_attendees']);
    return ok(mockEntities.attendance.length < before);
  },
} satisfies typeof Generated.commands;
