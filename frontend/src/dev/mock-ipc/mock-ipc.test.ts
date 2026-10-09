import { describe, expect, it } from 'vitest';

import { asMeetingId } from '@/features/meetings';
import type { Table } from '@/lib/ipc/generated/shared';

import { loadFixtureMeetings } from './fixtures';
import { legacyMeetingResponses } from './legacy-meetings';
import { MockDatabase } from './mock-database';

/** Every generated binding module and every mock module, by feature name. */
const generated = import.meta.glob<Record<string, unknown>>('../../lib/ipc/generated/*.ts', {
  eager: true,
});
const mocks = import.meta.glob<Record<string, unknown>>(['./*.ts', '!./*.test.ts'], {
  eager: true,
});

function byFeature(modules: Record<string, Record<string, unknown>>) {
  return new Map(
    Object.entries(modules).map(([path, module]) => [path.replace(/^.*\/|\.ts$/g, ''), module]),
  );
}

function database() {
  const changes: Table[][] = [];
  const db = new MockDatabase(loadFixtureMeetings(), (tables) => changes.push(tables));
  return { db, changes };
}

const ONE_ON_ONE = asMeetingId('meeting-fixture-one-on-one');
const NOW = new Date('2026-10-09T10:00:00Z');

describe('mock IPC modules', () => {
  it('mock every generated feature module with the same exports and commands', () => {
    const mocked = byFeature(mocks);
    for (const [feature, real] of byFeature(generated)) {
      if (feature === 'shared') continue;
      const mock = mocked.get(feature);
      expect(mock, `src/dev/mock-ipc/${feature}.ts`).toBeDefined();
      expect(Object.keys(mock ?? {}).sort()).toEqual(Object.keys(real).sort());
      expect(Object.keys(mock?.commands ?? {}).sort()).toEqual(
        Object.keys(real.commands ?? {}).sort(),
      );
    }
  });
});

describe('fixture loading', () => {
  it('validates all eight fixture meetings with unique ids', () => {
    const meetings = loadFixtureMeetings();

    expect(meetings).toHaveLength(8);
    expect(new Set(meetings.map(({ id }) => id)).size).toBe(8);
  });
});

describe('MockDatabase', () => {
  it('lists meetings newest first, like the SQL list query', () => {
    const titles = database()
      .db.meetings()
      .map(({ title }) => title);

    expect(titles[0]).toBe('Eng standup');
    expect(titles.at(-1)).toBe('Q4 planning');
  });

  it('renames with the Rust rules and reports the change as DbChanged tables', () => {
    const { db, changes } = database();

    const blank = db.rename(ONE_ON_ONE, '   ', NOW);
    const missing = db.rename(asMeetingId('meeting-x'), 'Sync', NOW);
    const renamed = db.rename(ONE_ON_ONE, '  Maya sync  ', NOW);

    expect(blank).toMatchObject({ status: 'error', error: { code: 'invalid' } });
    expect(missing).toMatchObject({ status: 'error', error: { code: 'not_found' } });
    expect(renamed).toMatchObject({
      status: 'ok',
      data: { title: 'Maya sync', updatedAt: NOW.toISOString() },
    });
    expect(changes).toEqual([['meetings']]);
  });

  it('reseeding restores renamed fixtures and reports what it wrote', () => {
    const { db } = database();
    db.rename(ONE_ON_ONE, 'Renamed', NOW);

    const report = db.reseed();

    expect(report.meetings).toBe(8);
    expect(db.meeting(ONE_ON_ONE)).toMatchObject({ data: { title: 'Shreyas / Maya 1:1' } });
  });
});

describe('legacy meeting commands', () => {
  it('page transcripts with the row ids and timing the Rust seeder writes', () => {
    const responses = legacyMeetingResponses(database().db);

    const page = responses.api_get_meeting_transcripts({
      meetingId: ONE_ON_ONE,
      limit: 2,
      offset: 0,
    });

    expect(page.has_more).toBe(true);
    expect(page.transcripts[0]).toMatchObject({
      id: 'meeting-fixture-one-on-one-segment-0000',
      timestamp: expect.stringMatching(/^\d\d:\d\d:\d\d$/) as unknown,
    });
  });

  it('fail for an unknown meeting the way the Rust command does', () => {
    const responses = legacyMeetingResponses(database().db);

    expect(() => responses.api_get_meeting_metadata({ meetingId: 'meeting-x' })).toThrow(
      'Meeting not found: meeting-x',
    );
  });
});
