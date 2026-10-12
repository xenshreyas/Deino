import { describe, expect, it } from 'vitest';

import type { MeetingId } from '@/features/meetings';
import { IDLE, type RecordingSnapshot } from '@/features/recording';

import { secondsLeft, TEST_SECONDS, testView } from './test-recording';

const meetingId = 'meeting-1' as MeetingId;
const busy = (phase: RecordingSnapshot['phase']): RecordingSnapshot => ({ ...IDLE, phase });

describe('the test recording view', () => {
  it('follows the recorder while it is busy, whatever happened before', () => {
    const done = { kind: 'done', meetingId } as const;

    expect(testView(busy('starting'), { kind: 'none' })).toEqual({ kind: 'recording' });
    expect(testView(busy('recording'), done)).toEqual({ kind: 'recording' });
    expect(testView(busy('stopping'), { kind: 'none' })).toEqual({ kind: 'saving' });
  });

  it('shows the outcome once the recorder is idle', () => {
    expect(testView(IDLE, { kind: 'none' })).toEqual({ kind: 'idle' });
    expect(testView(IDLE, { kind: 'done', meetingId })).toEqual({ kind: 'done', meetingId });
    expect(testView(IDLE, { kind: 'failed', message: 'No model' })).toEqual({
      kind: 'failed',
      message: 'No model',
    });
  });
});

describe('the countdown', () => {
  it('counts down from twenty seconds and stops at zero', () => {
    expect(secondsLeft(0)).toBe(TEST_SECONDS);
    expect(secondsLeft(5)).toBe(15);
    expect(secondsLeft(42)).toBe(0);
  });
});
