/**
 * Mock of `@/lib/ipc/generated/speakers`, over the fixture transcripts. The
 * standup starts with one speaker Deino is unsure of ("Is this Priya
 * Nair?") and one it does not know, and Maya Chen has a voice profile, so
 * the panel and Settings → People have something to show.
 */
import type * as Generated from '@/lib/ipc/generated/speakers';

import { mockPeople } from './database';
import { events } from './db';
import { loadFixtureMeetings } from './fixtures';
import { latency } from './latency';
import { ok } from './mock-database';
import { type MockPerson, MockSpeakers } from './mock-speakers';

/** The user's address in every fixture. */
const USER_EMAIL = 'shreyas@deino.dev';
const STANDUP = 'meeting-fixture-standup' as Generated.MeetingId;
const ONE_ON_ONE = 'meeting-fixture-one-on-one' as Generated.MeetingId;

let seq = 2_000_000;
const fixtures = loadFixtureMeetings();
const people: MockPerson[] = mockPeople.map((person) => ({
  id: person.id,
  name: person.name,
  email: person.email,
  isMe: person.email === USER_EMAIL,
}));
const speakers = new MockSpeakers(fixtures, people, (tables) => {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
});
for (const fixture of fixtures) speakers.voiceprints.titles.set(fixture.id, fixture.title);

/** The demo's starting state. */
function seedDemo() {
  const byEmail = (email: string) => people.find((person) => person.email === email);
  const priya = byEmail('priya@deino.dev');
  const maya = byEmail('maya@deino.dev');
  speakers.unname(
    STANDUP,
    'Speaker 4',
    priya === undefined ? null : { personId: priya.id, score: 0.66 },
  );
  speakers.unname(STANDUP, 'Speaker 5', null);
  if (maya !== undefined) {
    speakers.voiceprints.enrol(maya, ONE_ON_ONE, 48_000);
    speakers.voiceprints.enrol(maya, STANDUP, 31_000);
  }
}
seedDemo();

export const commands = {
  async participants(meetingId) {
    await latency();
    return ok(speakers.participants(meetingId));
  },
  async candidates(meetingId, query) {
    await latency();
    return ok(speakers.candidates(meetingId, query));
  },
  async rename(rename) {
    await latency();
    return speakers.rename(rename);
  },
  async merge(meetingId, from, into) {
    await latency();
    return speakers.merge(meetingId, from, into);
  },
  async confirmSuggestion(meetingId, speakerId) {
    await latency();
    return speakers.confirm(meetingId, speakerId);
  },
  async dismissSuggestion(speakerId) {
    await latency();
    return speakers.dismiss(speakerId);
  },
  async voiceProfiles() {
    await latency();
    return ok(speakers.voiceprints.profiles());
  },
  async deleteVoiceprint(id) {
    await latency();
    return speakers.voiceprints.delete(id);
  },
  async forgetVoice(personId) {
    await latency();
    return speakers.voiceprints.forget(personId);
  },
} satisfies typeof Generated.commands;
