import { describe, expect, it } from 'vitest';

import {
  longMeetingParticipants,
  longTranscript,
  PHRASE,
  PHRASE_EVERY,
} from '@/dev/mock-ipc/long-meeting';

import { segment, speaker } from '../test-data';
import { mergeLines } from './lines';
import { buildRows } from './rows';
import { buildSearchIndex, searchTranscript } from './search-index';
import type { SearchQuery } from './search-query';
import { SpeakerDirectory } from './speakers';

/** The search index over saved segments of the given speakers. */
function indexOf(speakers: ReturnType<typeof speaker>[], segments: ReturnType<typeof segment>[]) {
  const rows = buildRows(mergeLines(segments, new Map()), new SpeakerDirectory(speakers));
  return buildSearchIndex(rows);
}

const PRIYA = speaker('priya', 'Priya Shah');
const ME = speaker('me', 'Me', { isMe: true });

const index = indexOf(
  [PRIYA, ME],
  [
    segment('a', 0, 'The budget is fine. BUDGET again.', 'priya'),
    segment('b', 5, 'What about the école budget?', 'me'),
    segment('c', 9, 'Costs (in $) rose 3.5%.', 'priya'),
  ],
);

function search(text: string, speakers: string[] = []): ReturnType<typeof searchTranscript> {
  const query: SearchQuery = { text, speakers };
  return searchTranscript(index, query);
}

describe('searchTranscript', () => {
  it('finds every case-insensitive occurrence, in order, with its range', () => {
    const { hits, byRow } = search('budget');

    expect(hits.map((hit) => [hit.row, hit.range.start, hit.ordinal])).toEqual([
      [0, 4, 0],
      [0, 20, 1],
      [1, 21, 2],
    ]);
    expect(byRow.get(0)).toHaveLength(2);
    expect(byRow.has(2)).toBe(false);
  });

  it('matches Unicode without regard to case and treats symbols literally', () => {
    expect(search('ÉCOLE').hits).toHaveLength(1);
    expect(search('(in $)').hits[0]?.range).toEqual({ start: 6, end: 12 });
    expect(search('3.5%').hits).toHaveLength(1);
    expect(search('3x5').hits).toHaveLength(0);
  });

  it('limits hits to chosen speakers and to typed speaker: names', () => {
    expect(search('budget', ['me']).hits.map((hit) => hit.row)).toEqual([1]);
    expect(search('speaker:pri budget').hits.map((hit) => hit.row)).toEqual([0, 0]);
  });

  it('with only a speaker, marks each of their lines whole', () => {
    const { hits } = search('', ['priya']);

    expect(hits.map((hit) => hit.row)).toEqual([0, 2]);
    expect(hits[1]?.range).toEqual({ start: 0, end: 'Costs (in $) rose 3.5%.'.length });
  });

  it('finds nothing for an empty query', () => {
    expect(search('   ').hits).toEqual([]);
  });
});

describe('search performance', () => {
  /** A two-hour meeting at 5,000 lines (about one line every 1.4 s). */
  const meeting = longTranscript(5_000);
  const rows = buildRows(
    mergeLines(meeting.segments, new Map()),
    new SpeakerDirectory(longMeetingParticipants()),
  );

  it('indexes and searches a two-hour meeting in well under 100 ms per query', () => {
    const started = performance.now();
    const big = buildSearchIndex(rows);
    const indexed = performance.now() - started;
    const queries: SearchQuery[] = [
      { text: PHRASE, speakers: [] },
      { text: 'the', speakers: [] },
      { text: 'e', speakers: [] },
      { text: 'speaker:priya launch', speakers: [] },
      { text: 'no such words anywhere', speakers: [] },
    ];

    const timings = queries.map((query) => {
      const start = performance.now();
      searchTranscript(big, query);
      return performance.now() - start;
    });

    expect(indexed).toBeLessThan(100);
    expect(Math.max(...timings)).toBeLessThan(100);
    expect(searchTranscript(big, queries[0] ?? { text: '', speakers: [] }).hits).toHaveLength(
      5_000 / PHRASE_EVERY,
    );
  });
});
