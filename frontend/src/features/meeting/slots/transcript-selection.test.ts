import { describe, expect, it } from 'vitest';

import type { Transcript } from '@/features/transcript';

import { selectionSpan, timeAt } from './transcript-selection';

type Segment = Transcript['segments'][number];

const LINE = {
  id: '0192a5f0-0000-7000-8000-000000000001',
  start: 10_000,
  end: 13_000,
  text: 'Ship the beta Friday',
  words: [
    { text: 'Ship', start: 10_000, end: 10_400 },
    { text: 'the', start: 10_400, end: 10_600 },
    { text: 'beta', start: 10_600, end: 11_200 },
    { text: 'Friday', start: 11_900, end: 12_800 },
  ],
} as unknown as Segment;

describe('timeAt', () => {
  it('times a selection by the words it starts and ends in', () => {
    expect(timeAt(LINE, 5, 'start')).toBe(10_400);
    expect(timeAt(LINE, 13, 'end')).toBe(11_200);
  });

  it('falls back to the line without word timings', () => {
    const plain = { ...LINE, words: [] };

    expect(timeAt(plain, 5, 'start')).toBe(10_000);
    expect(timeAt(plain, 13, 'end')).toBe(13_000);
  });
});

describe('selectionSpan', () => {
  it('times the selected words of a bubble', () => {
    const container = document.createElement('div');
    container.innerHTML = `<div data-segment-id="${LINE.id}">${LINE.text}</div>`;
    document.body.replaceChildren(container);
    const text = container.firstChild?.firstChild;
    if (text == null) throw new Error('no text');
    const range = document.createRange();
    range.setStart(text, 5);
    range.setEnd(text, 13);

    const span = selectionSpan(container, range, new Map([[LINE.id, LINE]]));

    expect(span).toEqual({ start: 10_400, end: 11_200, text: 'the beta' });
  });
});
