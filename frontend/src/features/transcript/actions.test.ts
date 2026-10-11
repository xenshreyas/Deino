import { describe, expect, it } from 'vitest';

import { actionForKey, TRANSCRIPT_ACTIONS, type TranscriptActionContext } from './actions';
import { MEETING } from './test-data';

const context: TranscriptActionContext = {
  meetingId: MEETING,
  line: {
    id: 'line' as TranscriptActionContext['line']['id'],
    channel: 'mic',
    speakerId: null,
    start: 0,
    end: 0,
    text: 'hi',
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
};

describe('TRANSCRIPT_ACTIONS', () => {
  it('lists every action in toolbar order, with only Copy built so far', () => {
    expect(TRANSCRIPT_ACTIONS.map((action) => action.id)).toEqual([
      'play',
      'copy',
      'clip',
      'comment',
      'delete',
    ]);
    const shown = TRANSCRIPT_ACTIONS.filter((action) => action.state(context) !== 'hidden');
    expect(shown.map((action) => action.id)).toEqual(['copy']);
  });
});

describe('actionForKey', () => {
  it('runs Copy for ⌘C or Ctrl+C on a focused line, and nothing for a bare key', () => {
    expect(actionForKey(TRANSCRIPT_ACTIONS, { key: 'c', metaKey: true, ctrlKey: false })?.id).toBe(
      'copy',
    );
    expect(actionForKey(TRANSCRIPT_ACTIONS, { key: 'C', metaKey: false, ctrlKey: true })?.id).toBe(
      'copy',
    );
    expect(actionForKey(TRANSCRIPT_ACTIONS, { key: 'c', metaKey: false, ctrlKey: false })).toBe(
      undefined,
    );
  });
});
