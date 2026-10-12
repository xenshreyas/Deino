import { describe, expect, test } from 'vitest';

import { audioChange, keepsAtLeast, PERIODS, periodDays } from './periods';
import type { RetentionSettings } from './types';

const KEEP_ALL: RetentionSettings = { audio: 'never', transcript: 'never', everything: 'never' };

describe('retention periods', () => {
  test('never keeps longer than any number of days', () => {
    expect(keepsAtLeast('never', 'days365')).toBe(true);
    expect(keepsAtLeast('days365', 'never')).toBe(false);
    expect(keepsAtLeast('days30', 'days30')).toBe(true);
    expect(keepsAtLeast('days7', 'days30')).toBe(false);
  });

  test('every period counts its days', () => {
    expect(PERIODS.map(periodDays)).toEqual([null, 7, 30, 90, 365]);
  });

  test('a longer recording period raises the transcript with it', () => {
    const transcript30 = { ...KEEP_ALL, audio: 'days7', transcript: 'days30' } as const;

    expect(audioChange(transcript30, 'days90')).toEqual({
      audio: 'days90',
      transcript: 'days90',
    });
    expect(audioChange(transcript30, 'days30')).toEqual({ audio: 'days30' });
    expect(audioChange(transcript30, 'never')).toEqual({ audio: 'never', transcript: 'never' });
    expect(audioChange(KEEP_ALL, 'days7')).toEqual({ audio: 'days7' });
  });
});
