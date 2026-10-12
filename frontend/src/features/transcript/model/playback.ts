/**
 * The transcript and the player (TRN-03): which row is being played, and
 * the speaker bands the scrubber paints.
 */
import type { SpeakerBand } from '@/features/player';

import type { TranscriptRow } from '../types';
import { rowAtTime } from './rows';

/**
 * The row being said at `ms`: the last to start by then, while it lasts
 * (a pause after it marks nothing). `-1` when nothing is being said.
 */
export function rowPlayingAt(rows: readonly TranscriptRow[], ms: number): number {
  if (rows.length === 0) return -1;
  const index = rowAtTime(rows, ms);
  const line = rows[index]?.line;
  return line !== undefined && line.start <= ms && ms < Math.max(line.end, line.start + 1)
    ? index
    : -1;
}

/** Who spoke when, from the saved lines, for the scrubber. */
export function speakerBands(rows: readonly TranscriptRow[]): SpeakerBand[] {
  return rows
    .filter(({ line }) => !line.open && line.end > line.start)
    .map(({ line, speaker }) => ({ start: line.start, end: line.end, color: speaker.color }));
}
