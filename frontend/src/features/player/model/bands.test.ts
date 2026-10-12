import { describe, expect, it } from 'vitest';

import { BUCKETS, bandGradient, bucketColors } from './bands';

describe('bucketColors', () => {
  it('paints each slice with whoever spoke most of it, silence as 0', () => {
    const slices = bucketColors(
      [
        { start: 0, end: 250, color: 1 },
        { start: 250, end: 400, color: 3 },
        { start: 600, end: 1_000, color: 2 },
      ],
      1_000,
    );

    expect(slices).toHaveLength(BUCKETS);
    expect(slices[0]).toBe(1);
    expect(slices[Math.floor(BUCKETS * 0.3)]).toBe(3);
    expect(slices[Math.floor(BUCKETS * 0.5)]).toBe(0);
    expect(slices[BUCKETS - 1]).toBe(2);
  });

  it('is silent for an unknown length', () => {
    expect(bucketColors([{ start: 0, end: 10, color: 1 }], 0).every((c) => c === 0)).toBe(true);
  });
});

describe('bandGradient', () => {
  it('merges runs into stops over the speaker tokens', () => {
    const gradient = bandGradient([{ start: 0, end: 500, color: 4 }], 1_000);

    expect(gradient).toBe(
      'linear-gradient(to right, var(--speaker-4) 0% 50%, transparent 50% 100%)',
    );
  });
});
