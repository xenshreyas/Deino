/**
 * The fixtures' attendees as `people` rows, with the IDs the Rust seeder
 * gives them (`fixture_uuid("person/<email>")`), and the `people|search`
 * rules (name, word or email prefix, by name, at most eight).
 */
import type { Person, PersonId } from '@/lib/ipc/generated/people';

import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting } from './fixture-schema';

/** Same as `deino_core::people::SEARCH_LIMIT`. */
const SEARCH_LIMIT = 8;

/** Every attendee once, by email, sorted by name. */
export function fixturePeople(fixtures: readonly FixtureMeeting[]): Person[] {
  const byEmail = new Map<string, Person>();
  for (const fixture of fixtures) {
    const seededAt = fixture.startedAt;
    for (const attendee of fixture.attendees) {
      byEmail.set(attendee.email, {
        id: fixtureUuid(`person/${attendee.email}`) as PersonId,
        name: attendee.name,
        email: attendee.email,
        companyId: null,
        title: attendee.title,
        avatarPath: null,
        notesMd: '',
        summaryMd: null,
        summaryUpdatedAt: null,
        isMe: false,
        mergedInto: null,
        createdAt: seededAt,
        updatedAt: seededAt,
      });
    }
  }
  return [...byEmail.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** `people|search` over `people`. */
export function searchPeople(people: readonly Person[], query: string): Person[] {
  const prefix = query.trim().toLowerCase();
  return people
    .filter((person) => {
      const name = person.name.toLowerCase();
      return (
        name.startsWith(prefix) ||
        name.includes(` ${prefix}`) ||
        (person.email?.toLowerCase().startsWith(prefix) ?? false)
      );
    })
    .slice(0, SEARCH_LIMIT);
}
