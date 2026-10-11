/**
 * A row's height before it is measured, close enough that the scrollbar
 * barely moves when the real height arrives (ui.md Transcript bubble:
 * 14/20 text, 6px vertical padding, 2px between bubbles, 16px between
 * turns, a 20px header 4px above a turn's first bubble).
 */
import type { TranscriptRow } from '../types';

/** Characters per line of a bubble at 75% of the 720px column. */
const CHARS_PER_LINE = 62;
const LINE_HEIGHT_PX = 20;
const BUBBLE_PADDING_PX = 12;
const BUBBLE_GAP_PX = 2;
const TURN_GAP_PX = 16;
const HEADER_PX = 24;

/** The estimated height of `row` in pixels. */
export function estimateRowHeight(row: TranscriptRow | undefined): number {
  if (row === undefined) return LINE_HEIGHT_PX + BUBBLE_PADDING_PX;
  const lines = Math.max(1, Math.ceil(row.line.text.length / CHARS_PER_LINE));
  const bubble = lines * LINE_HEIGHT_PX + BUBBLE_PADDING_PX + BUBBLE_GAP_PX;
  return row.turnStart ? bubble + TURN_GAP_PX + HEADER_PX : bubble;
}
