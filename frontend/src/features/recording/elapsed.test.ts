import { describe, expect, it } from 'vitest';

import { elapsedSeconds, formatElapsed } from './elapsed';
import { IDLE } from './queries';

const STARTED = Date.parse('2026-10-09T10:00:00Z');

describe('elapsedSeconds', () => {
  it('counts from the start, less the pauses, and freezes while paused', () => {
    const recording = { ...IDLE, startedAt: '2026-10-09T10:00:00Z', pausedMs: 5_000 };

    expect(elapsedSeconds(recording, STARTED + 65_000)).toBe(60);
    expect(
      elapsedSeconds({ ...recording, pausedAt: '2026-10-09T10:00:30Z' }, STARTED + 99_000),
    ).toBe(25);
    expect(elapsedSeconds(IDLE, STARTED)).toBe(0);
  });
});

describe('formatElapsed', () => {
  it('shows minutes and seconds, then hours, never a negative time', () => {
    expect(formatElapsed(724.8)).toBe('12:04');
    expect(formatElapsed(3723)).toBe('1:02:03');
    expect(formatElapsed(-5)).toBe('0:00');
  });
});
