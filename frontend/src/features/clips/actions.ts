/**
 * What the transcript does with clips and moments: the bubble's Clip
 * action (the whole line) and the selection menu's Clip and Copy link (the
 * selected words), each timed by the words' own timestamps when the engine
 * gave them, else by the line's.
 */
import { copyMomentLink } from './moment-link';
import { startClip } from './store';
import type { MeetingId } from './types';

/** A stretch of the transcript, timed. */
export interface TimedSpan {
  /** Milliseconds from the start of the recording. */
  start: number;
  /** Milliseconds from the start of the recording. */
  end: number;
  /** The words, for the clip's suggested name. */
  text: string;
}

/** Opens the clip dialog for `span`. */
export function clipSpan(meetingId: MeetingId, span: TimedSpan): void {
  if (span.end <= span.start) return;
  startClip({ meetingId, start: span.start, end: span.end, suggestedName: span.text });
}

/** Copies a link to where `span` starts. */
export function copySpanLink(meetingId: MeetingId, span: TimedSpan): Promise<void> {
  return copyMomentLink(meetingId, span.start);
}
