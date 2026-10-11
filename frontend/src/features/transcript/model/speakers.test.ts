import { describe, expect, it } from 'vitest';

import { speaker } from '../test-data';
import type { TranscriptLine } from '../types';
import { SpeakerDirectory } from './speakers';

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
const PRIYA = speaker('speaker-priya', 'Priya Shah');
const UNKNOWN = speaker('speaker-2', null, { label: 'Speaker 2', color: 3 });

describe('SpeakerDirectory', () => {
  it('names a saved line after its participant, as the Participants panel does', () => {
    const directory = new SpeakerDirectory([ME, PRIYA]);

    const view = directory.of(line({ speakerId: PRIYA.speakerId }));

    expect(view).toMatchObject({ name: 'Priya Shah', isMe: false, unnamed: false, color: 2 });
  });

  it('shows an unnamed voice by its label and an unnamed microphone as You', () => {
    const directory = new SpeakerDirectory([speaker('mic', null, { isMe: true }), UNKNOWN]);

    expect(directory.of(line({ speakerId: UNKNOWN.speakerId }))).toMatchObject({
      name: 'Speaker 2',
      unnamed: true,
    });
    expect(directory.of(line({ speakerId: 'mic' as never })).name).toBe('You');
  });

  it('gives a live microphone line the user, and a system line the one other voice', () => {
    const directory = new SpeakerDirectory([ME, UNKNOWN]);

    expect(directory.of(line({ channel: 'mic' })).name).toBe('Shreyas');
    expect(directory.of(line({ channel: 'system' })).name).toBe('Speaker 2');
  });

  it('falls back to You and Them before the recorder stored its speakers', () => {
    const directory = new SpeakerDirectory([]);

    expect(directory.of(line({ channel: 'mic' }))).toMatchObject({ name: 'You', isMe: true });
    expect(directory.of(line({ channel: 'system' }))).toMatchObject({ name: 'Them', isMe: false });
    expect(directory.of(line({ channel: 'mixed' })).name).toBe('Speaker');
  });

  it('does not guess the other voice once diarization found several', () => {
    const directory = new SpeakerDirectory([ME, PRIYA, UNKNOWN]);

    expect(directory.of(line({ channel: 'system' })).name).toBe('Them');
    expect(directory.all.map((view) => view.name)).toEqual(['Shreyas', 'Priya Shah', 'Speaker 2']);
  });
});
