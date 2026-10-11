import { describe, expect, test } from 'vitest';

import { displayName, initials, speakerTone, talkTime } from './format';
import type { Participant, PersonId, SpeakerId } from './types';

function participant(change: Partial<Participant>): Participant {
  return {
    speakerId: 'speaker-1' as SpeakerId,
    label: 'Speaker 2',
    color: 2,
    isMe: false,
    person: null,
    speech: 0,
    sharePercent: 0,
    suggestion: null,
    ...change,
  };
}

describe('speaker formatting', () => {
  test('a speaker reads as their person, "You" on an unnamed microphone, else their label', () => {
    const maya = { id: 'p' as PersonId, name: 'Maya Chen' };
    expect(displayName(participant({ person: maya }))).toBe('Maya Chen');
    expect(displayName(participant({ isMe: true }))).toBe('You');
    expect(displayName(participant({}))).toBe('Speaker 2');
  });

  test('initials take two words, the number of an unnamed speaker, and whole characters', () => {
    expect(initials('Maya Chen')).toBe('MC');
    expect(initials('Maya Lin Chen')).toBe('ML');
    expect(initials('Speaker 12')).toBe('12');
    expect(initials('émile zola')).toBe('ÉZ');
    expect(initials('  ')).toBe('');
  });

  test('talk time reads in the largest useful units', () => {
    expect(talkTime(48_400)).toBe('48s');
    expect(talkTime(724_000)).toBe('12m 04s');
    expect(talkTime(3_720_000)).toBe('1h 02m');
  });

  test('palette slots map to literal utilities and cycle past six', () => {
    expect(speakerTone(2).fill).toBe('bg-speaker-2');
    expect(speakerTone(7).text).toBe('text-speaker-1');
  });
});
