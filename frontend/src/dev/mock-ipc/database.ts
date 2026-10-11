/**
 * The one in-memory database every mock module shares, loaded with the
 * fixture meetings, their notes and their people. Its writes become
 * `DbChanged` events, like SQLite commits.
 */
import { events } from './db';
import { loadFixtureMeetings } from './fixtures';
import {
  LONG_MEETING_ID,
  LONG_MEETING_TITLE,
  longTranscript,
  requestedLongMeeting,
} from './long-meeting';
import { MockDatabase } from './mock-database';
import { MockEnhanced } from './mock-enhanced';
import { MockNotes } from './mock-notes';
import { fixturePeople } from './mock-people';
import { MockTranscripts } from './mock-transcripts';

/** The commit sequence number, increasing by one per write. */
let seq = 0;

/** The fixture meetings every mock serves. */
export const fixtureMeetings = loadFixtureMeetings();

/** The shared mock database. */
export const mockDatabase = new MockDatabase(fixtureMeetings, (tables) => {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
});

/** The mock `meeting_notes` table (persisted per tab). */
export const mockNotes = new MockNotes(fixtureMeetings);

/** The mock `enhanced_notes` table (persisted per tab). */
export const mockEnhanced = new MockEnhanced(fixtureMeetings);

/** The fixtures' people, as the Rust seeder writes them. */
export const mockPeople = fixturePeople(fixtureMeetings);

/** The mock `transcript_segments` table. */
export const mockTranscripts = new MockTranscripts(fixtureMeetings);

const longMeetingLines = requestedLongMeeting();

/** The generated long meeting's transcript, when session storage asks for it (`long-meeting.ts`). */
export const longMeeting = longMeetingLines === null ? null : longTranscript(longMeetingLines);
if (longMeeting !== null) {
  const savedAt = new Date().toISOString();
  mockDatabase.insert({
    id: LONG_MEETING_ID,
    title: LONG_MEETING_TITLE,
    createdAt: savedAt,
    updatedAt: savedAt,
  });
  mockTranscripts.put(LONG_MEETING_ID, longMeeting);
}
