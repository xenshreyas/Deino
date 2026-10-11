/**
 * LEGACY SHIM, meeting page part: the untyped commands the inherited meeting
 * screen calls for metadata and transcript pages. Each one
 * moves to the `meetings` mock when ORG-07/TRN-01 port it to the plugin.
 *
 * Responses have the shapes of the Rust legacy commands in `api/api.rs`, and
 * transcript rows match what the Rust app serves for `deino_testkit::FixtureSeeder`'s
 * rows through the `transcripts` compatibility view (ADR 0012).
 */
import { fixtureUuid } from './fixture-ids';
import type { FixtureMeeting, FixtureSegment } from './fixture-schema';
import type { MockDatabase } from './mock-database';

/** Seconds, as the legacy `transcripts` view returns them. */
const seconds = (ms: number) => ms / 1000;

/** The view's `timestamp`: the offset into the recording as `HH:MM:SS`. */
function offsetClock(ms: number): string {
  const total = Math.floor(ms / 1000);
  const parts = [Math.floor(total / 3600), Math.floor(total / 60) % 60, total % 60];
  return parts.map((part) => String(part).padStart(2, '0')).join(':');
}

/** One legacy transcript row, as the compatibility view serves a seeded segment. */
function transcriptOf(meeting: FixtureMeeting, segment: FixtureSegment, index: number) {
  return {
    id: fixtureUuid(`segment/${meeting.id}/${String(index)}`),
    text: segment.text,
    timestamp: offsetClock(segment.startMs),
    audio_start_time: seconds(segment.startMs),
    audio_end_time: seconds(segment.endMs),
    duration: seconds(segment.endMs - segment.startMs),
  };
}

/** A string argument, or a rejection naming it (as Tauri's deserializer would). */
function stringArg(args: Record<string, unknown>, name: string): string {
  const value = args[name];
  if (typeof value !== 'string') throw new Error(`missing argument ${name}`);
  return value;
}

/** A numeric argument with a default. */
function numberArg(args: Record<string, unknown>, name: string, fallback: number): number {
  const value = args[name];
  return typeof value === 'number' ? value : fallback;
}

/** Arguments as the legacy `invoke` sent them. */
type Args = Record<string, unknown>;

/** The fixture named by the `meetingId` argument, or the Rust command's error. */
function findFixture(database: MockDatabase, args: Args): FixtureMeeting {
  const id = stringArg(args, 'meetingId');
  const fixture = database.fixture(id);
  if (!fixture) throw new Error(`Meeting not found: ${id}`);
  return fixture;
}

/** `api_get_meeting_metadata`: the meeting row, with its current title. */
function metadata(database: MockDatabase, args: Args) {
  const fixture = findFixture(database, args);
  const meeting = database.meetings().find(({ id }) => id === fixture.id);
  return {
    id: fixture.id,
    title: meeting?.title ?? fixture.title,
    created_at: meeting?.createdAt ?? fixture.startedAt,
    updated_at: meeting?.updatedAt ?? fixture.startedAt,
  };
}

/** `api_get_meeting_transcripts`: one page of transcript rows. */
function transcriptPage(database: MockDatabase, args: Args) {
  const fixture = findFixture(database, args);
  const offset = numberArg(args, 'offset', 0);
  const limit = numberArg(args, 'limit', 100);
  const rows = fixture.segments.map((segment, index) => transcriptOf(fixture, segment, index));
  const page = rows.slice(offset, offset + limit);
  return {
    transcripts: page,
    total_count: rows.length,
    has_more: offset + page.length < rows.length,
  };
}

/** `api_search_transcripts`: meetings whose transcript mentions the query (the ⌘K palette). */
function searchTranscripts(database: MockDatabase, args: Args) {
  const query = stringArg(args, 'query').toLowerCase();
  return database.meetings().flatMap((meeting) => {
    const hit = database
      .fixture(meeting.id)
      ?.segments.find((segment) => segment.text.toLowerCase().includes(query));
    if (hit === undefined) return [];
    return [{ id: meeting.id, title: meeting.title, matchContext: hit.text, timestamp: '' }];
  });
}

/** The legacy meeting-page commands over `database`. */
export function legacyMeetingResponses(database: MockDatabase) {
  return {
    api_get_meeting_metadata: (args: Args) => metadata(database, args),
    api_get_meeting_transcripts: (args: Args) => transcriptPage(database, args),
    api_search_transcripts: (args: Args) => searchTranscripts(database, args),
  };
}
