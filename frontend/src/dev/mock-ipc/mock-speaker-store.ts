/**
 * The one `MockSpeakers` every mock shares: the speakers mock answers the
 * `speakers` commands from it, and the transcript mock reads who said each
 * segment from it, so a rename in dev:mock moves the line on screen. The
 * standup starts with one speaker Deino is unsure of ("Is this Priya
 * Nair?") and one it does not know, and Maya Chen has a voice profile, so
 * the panel and Settings → People have something to show.
 */
import type * as Generated from '@/lib/ipc/generated/speakers';

import { events } from './db';
import { loadFixtureMeetings } from './fixtures';
import {
  LONG_MEETING_ID,
  longMeetingVoices,
  longTranscript,
  requestedLongMeeting,
} from './long-meeting';
import { fixturePeople } from './mock-people';
import { type MockPerson, MockSpeakers } from './mock-speakers';

/** The user's address in every fixture. */
const USER_EMAIL = 'shreyas@deino.dev';
const STANDUP = 'meeting-fixture-standup' as Generated.MeetingId;
const ONE_ON_ONE = 'meeting-fixture-one-on-one' as Generated.MeetingId;

let seq = 2_000_000;
const fixtures = loadFixtureMeetings();
// Derived here rather than read from `database.ts`, which (through feature
// code) can import this module while it is still initialising.
const mockPeople = fixturePeople(fixtures);
const longMeetingLines = requestedLongMeeting();
const longMeeting = longMeetingLines === null ? null : longTranscript(longMeetingLines);
const people: MockPerson[] = mockPeople.map((person) => ({
  id: person.id,
  name: person.name,
  email: person.email,
  isMe: person.email === USER_EMAIL,
}));
/** The shared speakers. */
export const mockSpeakers = new MockSpeakers(fixtures, people, (tables) => {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
});
for (const fixture of fixtures) mockSpeakers.voiceprints.titles.set(fixture.id, fixture.title);

/** The demo's starting state. */
function seedDemo() {
  const byEmail = (email: string) => people.find((person) => person.email === email);
  const priya = byEmail('priya@deino.dev');
  const maya = byEmail('maya@deino.dev');
  mockSpeakers.unname(
    STANDUP,
    'Speaker 4',
    priya === undefined ? null : { personId: priya.id, score: 0.66 },
  );
  mockSpeakers.unname(STANDUP, 'Speaker 5', null);
  if (maya !== undefined) {
    mockSpeakers.voiceprints.enrol(maya, ONE_ON_ONE, 48_000);
    mockSpeakers.voiceprints.enrol(maya, STANDUP, 31_000);
  }
}
seedDemo();

/** The generated long meeting's voices, when it was asked for. */
if (longMeeting !== null) {
  const said = longMeeting.segments.flatMap(({ id, speakerId, start, end }) =>
    speakerId === null ? [] : [{ segmentId: id, speakerId, length: end - start }],
  );
  mockSpeakers.seedMeeting(LONG_MEETING_ID, longMeetingVoices(), said);
}
