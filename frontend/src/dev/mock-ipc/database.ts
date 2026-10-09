/**
 * The one in-memory database every mock module shares, loaded with the
 * fixture meetings. Its writes become `DbChanged` events, like SQLite commits.
 */
import { events } from './db';
import { loadFixtureMeetings } from './fixtures';
import { MockDatabase } from './mock-database';

/** The commit sequence number, increasing by one per write. */
let seq = 0;

/** The shared mock database. */
export const mockDatabase = new MockDatabase(loadFixtureMeetings(), (tables) => {
  seq += 1;
  void events.dbChanged.emit({ tables, seq });
});
