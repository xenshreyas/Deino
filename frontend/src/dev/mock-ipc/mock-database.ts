/**
 * The in-memory "database" behind every mock module: the fixture meetings,
 * with the same rules and change signals as the Rust side.
 *
 * Reads return copies; writes validate like the Rust service, then report
 * the tables they touched through `onChange`, which the mock `db` module turns
 * into a `DbChanged` event, so TanStack Query invalidation works exactly as
 * it does in the app.
 */
import { asMeetingId, type Meeting, type MeetingId } from '@/features/meetings';
import type { SeedReport } from '@/lib/ipc/generated/dev';
import type { AppError, Table } from '@/lib/ipc/generated/shared';
import type { CommandResult } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import type { FixtureMeeting } from './fixture-schema';

/** Same limit as `deino_core::meetings::MAX_TITLE_CHARS`. */
const MAX_TITLE_CHARS = 200;

/** One meeting row and the fixture it came from (none for a scheduled note). */
interface Row {
  meeting: Meeting;
  fixture?: FixtureMeeting;
}

/** Unicode scalar values in `text`, which is what Rust's `chars().count()` counts. */
export function charCount(text: string): number {
  return text.match(/./gsu)?.length ?? 0;
}

/** Re-exported for the mocks that already import them from here. */
export { failure, ok };

/** The fixture meetings as mutable rows, with change notifications. */
export class MockDatabase {
  private rows = new Map<MeetingId, Row>();

  constructor(
    private readonly fixtures: readonly FixtureMeeting[],
    private readonly onChange: (tables: Table[]) => void,
  ) {
    this.load();
  }

  /** Every meeting, newest first (ties broken by id, like the SQL). */
  meetings(): Meeting[] {
    return [...this.rows.values()]
      .map(({ meeting }) => ({ ...meeting }))
      .sort((a, b) => b.createdAt.localeCompare(a.createdAt) || b.id.localeCompare(a.id));
  }

  /** One meeting, or `not_found`. */
  meeting(id: MeetingId): CommandResult<Meeting> {
    const row = this.rows.get(id);
    return row ? ok({ ...row.meeting }) : failure(notFound(id));
  }

  /** The fixture behind a meeting, for the legacy transcript commands. */
  fixture(id: string): FixtureMeeting | undefined {
    return [...this.rows.values()].find(({ meeting }) => meeting.id === id)?.fixture;
  }

  /** Renames a meeting with the Rust service's rules. */
  rename(id: MeetingId, input: string, now: Date): CommandResult<Meeting> {
    const row = this.rows.get(id);
    const title = input.trim();
    if (title === '') return failure(invalid('a meeting title cannot be empty'));
    if (charCount(title) > MAX_TITLE_CHARS) {
      return failure(invalid(`a meeting title can be at most ${MAX_TITLE_CHARS} characters long`));
    }
    if (!row) return failure(notFound(id));
    row.meeting = { ...row.meeting, title, updatedAt: now.toISOString() };
    this.onChange(['meetings']);
    return ok({ ...row.meeting });
  }

  /** Adds a scheduled meeting for notes written ahead (`notes|create_scheduled`). */
  addScheduled(input: string, now: Date): CommandResult<Meeting> {
    const title = input.trim();
    if (title === '') return failure(invalid('a meeting title cannot be empty'));
    const meeting: Meeting = {
      id: asMeetingId(`meeting-${crypto.randomUUID()}`),
      title,
      createdAt: now.toISOString(),
      updatedAt: now.toISOString(),
    };
    this.rows.set(meeting.id, { meeting });
    this.onChange(['meetings']);
    return ok({ ...meeting });
  }

  /** Adds a ready-made meeting row (the generated long meeting), without a change signal. */
  insert(meeting: Meeting): void {
    this.rows.set(meeting.id, { meeting: { ...meeting } });
  }

  /** Reports a write to `tables`, as a commit would. */
  touch(tables: Table[]): void {
    this.onChange(tables);
  }

  /** Restores every fixture meeting, like the Rust `FixtureSeeder`. */
  reseed(): SeedReport {
    this.load();
    this.onChange([
      'calendar_accounts',
      'calendar_events',
      'calendars',
      'meetings',
      'companies',
      'people',
      'meeting_attendees',
      'speakers',
      'transcript_segments',
      'meeting_notes',
    ]);
    const segments = this.fixtures.reduce((sum, fixture) => sum + fixture.segments.length, 0);
    return { meetings: this.fixtures.length, segments };
  }

  private load(): void {
    this.rows = new Map(this.fixtures.map((fixture) => [asMeetingId(fixture.id), rowOf(fixture)]));
  }
}

/** The row the seeder writes: saved when recording stopped. */
function rowOf(fixture: FixtureMeeting): Row {
  const savedAt = new Date(Date.parse(fixture.startedAt) + fixture.durationMs).toISOString();
  const meeting: Meeting = {
    id: asMeetingId(fixture.id),
    title: fixture.title,
    createdAt: savedAt,
    updatedAt: savedAt,
  };
  return { meeting, fixture };
}

function notFound(id: MeetingId): AppError {
  return { code: 'not_found', message: `meeting ${id} was not found` };
}

function invalid(message: string): AppError {
  return { code: 'invalid', message };
}
