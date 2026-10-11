import { beforeEach, describe, expect, it } from 'vitest';

import { asMeetingId } from '@/features/meetings';

import { loadFixtureMeetings } from './fixtures';
import { MockEnhanced } from './mock-enhanced';

const ONE_ON_ONE = asMeetingId('meeting-fixture-one-on-one');
const STANDUP = asMeetingId('meeting-fixture-standup');
const NOW = new Date('2026-10-11T10:00:00Z');

describe('MockEnhanced', () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it('seeds notes for the fixtures, except the two that show the empty state', () => {
    const notes = new MockEnhanced(loadFixtureMeetings());

    expect(notes.view(ONE_ON_ONE, null, null)?.version.version).toBe(1);
    expect(notes.view(STANDUP, null, null)).toBeNull();
  });

  it("keeps every line the user typed, as the user's", () => {
    const fixture = loadFixtureMeetings().find((meeting) => meeting.id === ONE_ON_ONE);
    const notes = new MockEnhanced(loadFixtureMeetings());

    const json = JSON.stringify(notes.view(ONE_ON_ONE, null, null)?.doc);

    expect(fixture?.userNotes).toContain('send her PM draft thurs');
    expect(json).toContain(
      '"text":"send her PM draft thurs","marks":[{"type":"origin","attrs":{"author":"user"',
    );
  });

  it('marks a saved edit and refuses edits to older versions', () => {
    const notes = new MockEnhanced(loadFixtureMeetings());
    const doc = notes.view(ONE_ON_ONE, null, null)?.doc;
    if (doc === undefined) throw new Error('no notes');

    expect(notes.save(ONE_ON_ONE, 1, doc, NOW)).toBe(true);
    expect(notes.isEdited(ONE_ON_ONE)).toBe(true);
    notes.write(ONE_ON_ONE, NOW);

    expect(notes.save(ONE_ON_ONE, 1, doc, NOW)).toBe(false);
    expect(notes.versions(ONE_ON_ONE).map((info) => [info.version, info.isUserEdited])).toEqual([
      [2, false],
      [1, true],
    ]);
  });

  it('asks to regenerate once the user notes changed after the newest version', () => {
    const notes = new MockEnhanced(loadFixtureMeetings());
    const written = notes.view(ONE_ON_ONE, null, null)?.version.createdAt ?? '';

    expect(notes.view(ONE_ON_ONE, null, written)?.reasons).toEqual([]);
    expect(notes.view(ONE_ON_ONE, null, '2099-01-01T00:00:00Z')?.reasons).toEqual(['notesEdited']);
  });
});
