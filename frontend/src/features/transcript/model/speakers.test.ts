import { describe, expect, it } from 'vitest';

import { speaker } from '../test-data';
import type { TranscriptLine } from '../types';
import { initials, SpeakerDirectory, speakerTextClass, speakerTintClass } from './speakers';

function line(extra: Partial<TranscriptLine>): TranscriptLine {
  return {
    id: 'line' as TranscriptLine['id'],
    channel: 'system',
    speakerId: null,
    start: 0,
    end: 0,
    text: 'hi',
    open: false,
    ...extra,
  };
}

const ME = speaker('speaker-me', 'Shreyas', { isMe: true, color: 1 });
const PRIYA = speaker('speaker-priya', 'Priya Shah', { personId: 'person-priya' as never });
const UNKNOWN = speaker('speaker-2', 'Speaker 2', { color: 3 });

describe('SpeakerDirectory', () => {
  it('names a saved line by its stored speaker', () => {
    const directory = new SpeakerDirectory([ME, PRIYA]);

    const view = directory.of(line({ speakerId: PRIYA.id }));

    expect(view).toMatchObject({ name: 'Priya Shah', isMe: false, identified: true, color: 2 });
  });

  it('gives a live microphone line the user, and a system line the one other voice', () => {
    const directory = new SpeakerDirectory([ME, UNKNOWN]);

    expect(directory.of(line({ channel: 'mic' })).name).toBe('Shreyas');
    expect(directory.of(line({ channel: 'system' }))).toMatchObject({
      name: 'Speaker 2',
      identified: false,
    });
  });

  it('falls back to Me and Them before the recorder stored its speakers', () => {
    const directory = new SpeakerDirectory([]);

    expect(directory.of(line({ channel: 'mic' }))).toMatchObject({ name: 'Me', isMe: true });
    expect(directory.of(line({ channel: 'system' }))).toMatchObject({ name: 'Them', isMe: false });
    expect(directory.of(line({ channel: 'mixed' })).name).toBe('Speaker');
  });

  it('does not guess the other voice once diarization found several', () => {
    const directory = new SpeakerDirectory([ME, PRIYA, UNKNOWN]);

    expect(directory.of(line({ channel: 'system' })).name).toBe('Them');
    expect(directory.all.map((view) => view.name)).toEqual(['Shreyas', 'Priya Shah', 'Speaker 2']);
  });
});

describe('speaker colours and initials', () => {
  it('maps every palette slot to a token class and falls back to the first', () => {
    expect(speakerTextClass(4)).toBe('text-speaker-4');
    expect(speakerTintClass(6)).toBe('bg-speaker-6/16');
    expect(speakerTextClass(9)).toBe('text-speaker-1');
  });

  it('takes the first and last initials, whole characters only', () => {
    expect(initials('Priya Shah')).toBe('PS');
    expect(initials('Maya Lin Chen')).toBe('MC');
    expect(initials('me')).toBe('M');
    expect(initials('👩🏽‍💻 Dev')).toBe('👩🏽‍💻D');
    expect(initials('  ')).toBe('');
  });
});
