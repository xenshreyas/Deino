import { describe, expect, it } from 'vitest';

import { locateQuote, spanContains } from './anchor';

const LINE = 'The pricing page needs a rewrite before launch.';
const PRICING = { start: 4, end: 16, quote: 'pricing page' };

describe('locateQuote', () => {
  it('keeps the span while its words are where they were', () => {
    expect(locateQuote(LINE, PRICING)).toEqual({ start: 4, end: 16 });
  });

  it('follows its words when an edit earlier in the line moves them', () => {
    const edited = 'Honestly, the pricing page needs a rewrite before launch.';

    expect(locateQuote(edited, PRICING)).toEqual({ start: 14, end: 26 });
  });

  it('takes the occurrence nearest the old place when the words repeat', () => {
    const text = 'pricing page, then the pricing page again';

    expect(locateQuote(text, { start: 21, end: 33, quote: 'pricing page' })).toEqual({
      start: 23,
      end: 35,
    });
  });

  it('is detached (null) once an edit removed its words', () => {
    expect(locateQuote('The plans page needs a rewrite.', PRICING)).toBeNull();
    expect(locateQuote(LINE, { start: 0, end: 0, quote: '' })).toBeNull();
  });
});

describe('spanContains', () => {
  it('includes the start and excludes the end', () => {
    expect(spanContains({ start: 4, end: 16 }, 4)).toBe(true);
    expect(spanContains({ start: 4, end: 16 }, 16)).toBe(false);
  });
});
