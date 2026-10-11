/**
 * Lines as list rows, grouped into speaker turns (ui.md Transcript bubble):
 * consecutive lines of one speaker belong to one turn while each starts
 * within a minute of the previous one ending. The first row of a turn
 * carries the speaker's name and time.
 */
import type { TranscriptLine, TranscriptRow } from '../types';
import type { SpeakerDirectory } from './speakers';

/** A pause longer than this starts a new turn even for the same speaker. */
export const TURN_GAP_MS = 60_000;

/** One row per line, in order, with turn starts marked. */
export function buildRows(
  lines: readonly TranscriptLine[],
  speakers: SpeakerDirectory,
): TranscriptRow[] {
  const rows: TranscriptRow[] = [];
  let previous: TranscriptRow | undefined;
  for (const line of lines) {
    const speaker = speakers.of(line);
    const turnStart =
      previous === undefined ||
      previous.speaker.key !== speaker.key ||
      line.start - Math.max(previous.line.end, previous.line.start) > TURN_GAP_MS;
    previous = { line, speaker, turnStart };
    rows.push(previous);
  }
  return rows;
}

/** The row of the line being said at `ms`: the last that starts by then (0 before any). */
export function rowAtTime(rows: readonly TranscriptRow[], ms: number): number {
  let low = 0;
  let high = rows.length - 1;
  let found = 0;
  while (low <= high) {
    const middle = Math.floor((low + high) / 2);
    const start = rows[middle]?.line.start ?? 0;
    if (start <= ms) {
      found = middle;
      low = middle + 1;
    } else {
      high = middle - 1;
    }
  }
  return found;
}
