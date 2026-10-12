import { describe, expect, it, vi } from 'vitest';

import { linksCopy } from './copy';
import { follow, followAll, type LinkTargets } from './follow';
import type { CalendarEventId, MeetingId, Navigation } from './types';

const MEETING = 'meeting-fixture-standup' as MeetingId;
const EVENT = '0190f5a2-7c1e-7b3a-9d2e-5f6a7b8c9d0e' as CalendarEventId;

function fakeTargets(eventNote: () => Promise<void> = () => Promise.resolve()) {
  return {
    openMeeting: vi.fn<LinkTargets['openMeeting']>(),
    openNewNote: vi.fn<LinkTargets['openNewNote']>(),
    openEventNote: vi.fn<LinkTargets['openEventNote']>(eventNote),
    openSearch: vi.fn<LinkTargets['openSearch']>(),
    openAsk: vi.fn<LinkTargets['openAsk']>(),
    notify: vi.fn<LinkTargets['notify']>(),
  } satisfies LinkTargets;
}

describe('follow', () => {
  it('opens a meeting at its moment', async () => {
    const targets = fakeTargets();
    await follow({ kind: 'meeting', meetingId: MEETING, atMs: 231259 }, targets);
    expect(targets.openMeeting).toHaveBeenCalledWith(MEETING, 231259);
  });

  it('opens the new-note screen, or the event note when an event is given', async () => {
    const targets = fakeTargets();
    await follow({ kind: 'new_note', eventId: null }, targets);
    expect(targets.openNewNote).toHaveBeenCalledOnce();

    await follow({ kind: 'new_note', eventId: EVENT }, targets);
    expect(targets.openEventNote).toHaveBeenCalledWith(EVENT);
    expect(targets.openNewNote).toHaveBeenCalledOnce();
  });

  it('falls back to a plain new note when the event cannot be opened', async () => {
    const targets = fakeTargets(() => Promise.reject(new Error('not found')));
    await follow({ kind: 'new_note', eventId: EVENT }, targets);
    expect(targets.notify).toHaveBeenCalledWith(linksCopy.eventNoteFailed);
    expect(targets.openNewNote).toHaveBeenCalledOnce();
  });

  it('opens a search', async () => {
    const targets = fakeTargets();
    await follow({ kind: 'search', query: 'release cut' }, targets);
    expect(targets.openSearch).toHaveBeenCalledWith('release cut');
  });

  it('says Recipes is not here yet for a recipe', async () => {
    const targets = fakeTargets();
    await follow({ kind: 'recipe' }, targets);
    expect(targets.notify).toHaveBeenCalledWith(
      linksCopy.recipeWip,
      linksCopy.recipeWipDescription,
    );
  });

  it('opens the Ask panel', async () => {
    const targets = fakeTargets();
    await follow({ kind: 'ask' }, targets);
    expect(targets.openAsk).toHaveBeenCalledOnce();
  });

  it('follows several in order', async () => {
    const targets = fakeTargets();
    const navigations: Navigation[] = [
      { kind: 'search', query: 'a' },
      { kind: 'search', query: 'b' },
    ];
    await followAll(navigations, targets);
    expect(targets.openSearch.mock.calls).toEqual([['a'], ['b']]);
  });
});
