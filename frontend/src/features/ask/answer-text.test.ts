import { describe, expect, it } from 'vitest';

import type { AnswerPart } from './types';
import { plainText, slashQuery, streamingText, typedRecipe } from './answer-text';

describe('streamingText', () => {
  it('hides complete and half-arrived markers', () => {
    expect(streamingText('Beta holds [s2]. Pricing [s3, S4 00:04] is open')).toBe(
      'Beta holds. Pricing is open',
    );
    expect(streamingText('Beta holds [s1')).toBe('Beta holds');
    expect(streamingText('Beta holds [')).toBe('Beta holds');
  });

  it('keeps brackets that are not markers', () => {
    expect(streamingText('The code was [redacted].')).toBe('The code was [redacted].');
  });
});

describe('plainText', () => {
  it('joins the runs without citations', () => {
    const parts: AnswerPart[] = [
      {
        text: 'Beta holds',
        citations: [{ segmentId: 'a' as AnswerPart['citations'][number]['segmentId'], start: 1 }],
      },
      { text: '. Done.', citations: [] },
    ];
    expect(plainText(parts)).toBe('Beta holds. Done.');
  });
});

describe('slash commands', () => {
  it('reads the slug being typed', () => {
    expect(slashQuery('/')).toBe('');
    expect(slashQuery('/Write-f')).toBe('write-f');
    expect(slashQuery('/write follow')).toBeNull();
    expect(slashQuery('what /now')).toBeNull();
  });

  it('splits a typed recipe from what the user adds', () => {
    expect(typedRecipe('/coach-me  focus on pacing ')).toEqual({
      slug: 'coach-me',
      rest: 'focus on pacing',
    });
    expect(typedRecipe('/coach-me')).toEqual({ slug: 'coach-me', rest: '' });
    expect(typedRecipe('coach me')).toBeNull();
  });
});
