import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createQueryClient } from '@/lib/ipc';

import { actionForKey, TRANSCRIPT_ACTIONS, type TranscriptActionContext } from './actions';
import { useEditingStore } from './edit/editing-store';
import { MEETING } from './test-data';

const player = vi.hoisted(() => ({ playFrom: vi.fn<(meeting: string, ms: number) => void>() }));
vi.mock('@/features/player', () => ({ playerActions: { playFrom: player.playFrom } }));

const context = (extra: Partial<TranscriptActionContext> = {}): TranscriptActionContext => ({
  meetingId: MEETING,
  line: {
    id: 'line' as TranscriptActionContext['line']['id'],
    channel: 'mic',
    speakerId: null,
    start: 12_500,
    end: 14_000,
    text: 'hi',
    words: [],
    editedAt: null,
    open: false,
  },
  speaker: {
    key: 'me',
    speakerId: null,
    name: 'Me',
    isMe: true,
    color: 1,
    unnamed: false,
  },
  client: createQueryClient(),
  playable: false,
  ...extra,
});

/** The ids of the actions shown for `ctx`. */
function shown(ctx: TranscriptActionContext): string[] {
  return TRANSCRIPT_ACTIONS.filter((action) => action.state(ctx) !== 'hidden').map((a) => a.id);
}

beforeEach(() => {
  player.playFrom.mockClear();
  useEditingStore.setState({ editing: null });
});

describe('TRANSCRIPT_ACTIONS', () => {
  it('lists every action in toolbar order', () => {
    expect(TRANSCRIPT_ACTIONS.map((action) => action.id)).toEqual([
      'play',
      'copy',
      'edit',
      'clip',
      'comment',
      'delete',
    ]);
  });

  it('offers Play only with a recording, and only Copy on a line still being spoken', () => {
    const saved = context();
    const live = context({ line: { ...saved.line, open: true }, playable: true });

    expect(shown(saved)).toEqual(['copy', 'edit', 'delete']);
    expect(shown(context({ playable: true }))).toEqual(['play', 'copy', 'edit', 'delete']);
    expect(shown(live)).toEqual(['copy']);
  });

  it('plays from the line start and edits in place', async () => {
    const ctx = context({ playable: true });

    await TRANSCRIPT_ACTIONS.find((action) => action.id === 'play')?.run(ctx);
    await TRANSCRIPT_ACTIONS.find((action) => action.id === 'edit')?.run(ctx);

    expect(player.playFrom).toHaveBeenCalledWith(MEETING, 12_500);
    expect(useEditingStore.getState().editing).toBe(ctx.line.id);
  });
});

describe('actionForKey', () => {
  it('maps ⌘C to Copy, ↵ to Edit and ⌘⌫ to Delete, and nothing to a bare key', () => {
    const key = (k: string, metaKey: boolean) =>
      actionForKey(TRANSCRIPT_ACTIONS, { key: k, metaKey, ctrlKey: false })?.id;

    expect(key('c', true)).toBe('copy');
    expect(actionForKey(TRANSCRIPT_ACTIONS, { key: 'C', metaKey: false, ctrlKey: true })?.id).toBe(
      'copy',
    );
    expect(key('Enter', false)).toBe('edit');
    expect(key('Backspace', true)).toBe('delete');
    expect(key('c', false)).toBe(undefined);
    expect(key('Backspace', false)).toBe(undefined);
  });
});
