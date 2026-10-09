import { describe, expect, it } from 'vitest';

import { millisecondsLeft, secondsLeft } from './countdown';

const DEADLINE = '2026-10-09T10:00:10.000Z';
const at = (iso: string) => Date.parse(iso);

describe('auto-start countdown', () => {
  it('rounds partial seconds up, so the label never shows 0 before recording starts', () => {
    expect(secondsLeft(DEADLINE, at('2026-10-09T10:00:00.000Z'))).toBe(10);
    expect(secondsLeft(DEADLINE, at('2026-10-09T10:00:09.100Z'))).toBe(1);
    expect(secondsLeft(DEADLINE, at('2026-10-09T10:00:10.000Z'))).toBe(0);
  });

  it('never goes negative once the deadline has passed', () => {
    expect(secondsLeft(DEADLINE, at('2026-10-09T10:01:00.000Z'))).toBe(0);
    expect(millisecondsLeft(DEADLINE, at('2026-10-09T10:01:00.000Z'))).toBe(0);
    expect(millisecondsLeft(DEADLINE, at('2026-10-09T10:00:04.000Z'))).toBe(6_000);
  });
});
