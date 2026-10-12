/**
 * The "Try it" step's 20-second test recording, as pure rules over the
 * recorder's snapshot and what the step itself saw.
 */
import type { MeetingId } from '@/features/meetings';
import type { RecordingSnapshot } from '@/features/recording';

/** How long the test records before it stops itself. */
export const TEST_SECONDS = 20;

/** What the step remembers between recorder updates. */
export type TestOutcome =
  /** Nothing tried yet, or a failure was dismissed. */
  | { kind: 'none' }
  /** Start or stop failed. */
  | { kind: 'failed'; message: string }
  /** Stopped; the meeting is being transcribed and summarised. */
  | { kind: 'done'; meetingId: MeetingId };

/** What the step shows. */
export type TestView =
  | { kind: 'idle' }
  | { kind: 'recording' }
  | { kind: 'saving' }
  | { kind: 'failed'; message: string }
  | { kind: 'done'; meetingId: MeetingId };

/**
 * The step's view: the recorder decides while it is busy; afterwards the
 * outcome of the test does.
 */
export function testView(snapshot: RecordingSnapshot, outcome: TestOutcome): TestView {
  switch (snapshot.phase) {
    case 'starting':
    case 'recording':
    case 'paused':
      return { kind: 'recording' };
    case 'stopping':
      return { kind: 'saving' };
    case 'idle':
      return outcome.kind === 'none' ? { kind: 'idle' } : outcome;
  }
}

/** Seconds left of the test after `elapsed` seconds, never below zero. */
export function secondsLeft(elapsed: number): number {
  return Math.max(0, TEST_SECONDS - elapsed);
}
