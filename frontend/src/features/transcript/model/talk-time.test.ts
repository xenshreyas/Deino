import { describe, expect, it } from 'vitest';

import { segment, speaker } from '../test-data';
import type { Segment } from '../types';
import { mergeLines } from './lines';
import { buildRows } from './rows';
import { SpeakerDirectory } from './speakers';
import { talkShares, wholePercents } from './talk-time';

const directory = new SpeakerDirectory([
  speaker('me', 'Me', { isMe: true, color: 1 }),
  speaker('ana', 'Ana', { color: 2 }),
  speaker('ben', 'Ben', { color: 3 }),
]);

/** A segment said by `who` from `start` to `end` seconds. */
function said(id: string, who: string | null, start: number, end: number): Segment {
  return { ...segment(id, start, 'words', who), end: end * 1000 };
}

function shares(...segments: Segment[]) {
  return talkShares(buildRows(mergeLines(segments, new Map()), directory)).map((share) => ({
    name: share.speaker.name,
    ms: share.ms,
    percent: share.percent,
  }));
}

describe('talkShares', () => {
  it('sums each speaker’s line lengths, most talk time first', () => {
    expect(
      shares(said('a', 'me', 0, 10), said('b', 'ana', 10, 40), said('c', 'me', 40, 50)),
    ).toEqual([
      { name: 'Ana', ms: 30_000, percent: 60 },
      { name: 'Me', ms: 20_000, percent: 40 },
    ]);
  });

  it('rounds the shares so they add up to exactly 100', () => {
    const result = shares(
      said('a', 'me', 0, 10),
      said('b', 'ana', 10, 20),
      said('c', 'ben', 20, 30),
    );

    expect(result.map((share) => share.percent).reduce((sum, value) => sum + value, 0)).toBe(100);
    expect(result.map((share) => share.percent)).toEqual([34, 33, 33]);
  });

  it('skips lines without a length', () => {
    expect(shares(said('a', 'me', 5, 5), said('b', 'ana', 0, 4))).toEqual([
      { name: 'Ana', ms: 4000, percent: 100 },
    ]);
  });

  it('is empty without speech', () => {
    expect(shares()).toEqual([]);
  });
});

describe('wholePercents', () => {
  it('gives zero to every value when nothing was said', () => {
    expect(wholePercents([0, 0])).toEqual([0, 0]);
  });

  it('rounds the largest remainders up', () => {
    expect(wholePercents([1, 1, 1, 1, 1, 1])).toEqual([17, 17, 17, 17, 16, 16]);
  });
});
