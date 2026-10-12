import { describe, expect, it } from 'vitest';

import { byKind, changedWords, phrases, termInput } from './entries';
import type { VocabTerm, VocabTermId } from './types';

function entry(kind: VocabTerm['kind'], text: string): VocabTerm {
  return {
    id: text as VocabTermId,
    kind,
    text,
    soundsLike: [],
    createdAt: '2026-10-11T09:00:00Z',
    updatedAt: '2026-10-11T09:00:00Z',
  };
}

describe('phrases', () => {
  it('splits on commas, trims and drops blanks', () => {
    expect(phrases(' Sreyas,  Shrey   as ,, ')).toEqual(['Sreyas', 'Shrey as']);
    expect(phrases('')).toEqual([]);
  });
});

describe('termInput', () => {
  it('turns the two fields into what Rust validates', () => {
    expect(termInput('replacement', ' going to ', 'gonna, gunna')).toEqual({
      kind: 'replacement',
      text: 'going to',
      soundsLike: ['gonna', 'gunna'],
    });
  });
});

describe('byKind', () => {
  it('keeps each kind in order', () => {
    const split = byKind([entry('replacement', 'going to'), entry('term', 'Deino')]);

    expect(split.terms.map((t) => t.text)).toEqual(['Deino']);
    expect(split.replacements.map((t) => t.text)).toEqual(['going to']);
  });
});

describe('changedWords', () => {
  it('names only the words that changed', () => {
    expect(changedWords('Thanks Sreyas. Bye', 'Thanks Shreyas. Bye')).toEqual({
      from: 'Sreyas.',
      to: 'Shreyas.',
    });
    expect(changedWords('gonna ship', 'going to ship')).toEqual({ from: 'gonna', to: 'going to' });
  });

  it('copes with a correction at either end or the whole line', () => {
    expect(changedWords('Sreyas', 'Shreyas')).toEqual({ from: 'Sreyas', to: 'Shreyas' });
    expect(changedWords('a b', 'a b')).toEqual({ from: '', to: '' });
  });
});
