import { describe, expect, it } from 'vitest';

import { timedWords, wordAt } from './word-ranges';

const word = (text: string, start: number) => ({ text, start, end: start + 300 });

describe('timedWords', () => {
  it('finds each word in order, with its time', () => {
    const words = timedWords('Ship it, Monday.', [
      word('Ship', 0),
      word('it,', 400),
      word('Monday.', 800),
    ]);

    expect(words?.map(({ range }) => range)).toEqual([
      { start: 0, end: 4 },
      { start: 5, end: 8 },
      { start: 9, end: 16 },
    ]);
    expect(words?.[2]?.start).toBe(800);
  });

  it('gives up when the words no longer spell the text (an edited line)', () => {
    expect(timedWords('Ship it Friday.', [word('Ship', 0), word('Monday.', 800)])).toBeNull();
    expect(timedWords('Anything', [])).toBeNull();
  });
});

describe('wordAt', () => {
  it('is the last word started by then', () => {
    const words = timedWords('a b c', [word('a', 0), word('b', 500), word('c', 1_000)]) ?? [];

    expect(wordAt(words, 0)).toBe(0);
    expect(wordAt(words, 999)).toBe(1);
    expect(wordAt(words, 5_000)).toBe(2);
  });
});
