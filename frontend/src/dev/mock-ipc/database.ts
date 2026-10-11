/**
 * The one in-memory database every mock module shares, loaded with the
 * fixture meetings, their notes and their people. Its writes become
 * `DbChanged` events, like SQLite commits.
 */
import { events } from './db';
import { loadFixtureMeetings } from './fixtures';
import { MockDatabase } from './mock-database';
import { MockEnhanced } from './mock-enhanced';
import { MockNotes } from './mock-notes';
import { fixturePeople } from './mock-people';

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
export const mockEnhanced = new MockEnhanced(fixtures);

/** The fixtures' people, as the Rust seeder writes them. */
export const mockPeople = fixturePeople(fixtureMeetings);
