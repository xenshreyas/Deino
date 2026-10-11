/**
 * A deep link's moment: a time (`?t=<ms>`: action items, ORG-08 search) or
 * one line (`?segment=<id>`: NOT-03 "Show in transcript"). On mount it
 * scrolls that line into the middle of the view and marks it: highlighted
 * for a moment, and `aria-current="location"` until another moment comes.
 * Render it with `key={momentKey(moment)}`, so each new moment remounts it
 * and only it.
 */
import { useMountEffect } from '@/hooks/useMountEffect';

import { rowAtTime } from '../model/rows';
import type { TranscriptMoment, TranscriptRow } from '../types';

/** How long the moment stays marked before it fades back. */
export const MOMENT_FLASH_MS = 1_600;

/** A key that changes whenever the moment does. */
export function momentKey(moment: TranscriptMoment): string {
  return moment.kind === 'time' ? `time:${String(moment.at)}` : `line:${moment.segmentId}`;
}

/** The row a moment points at, or `-1` for a line that is not there. */
export function rowOfMoment(rows: readonly TranscriptRow[], moment: TranscriptMoment): number {
  if (moment.kind === 'time') return rows.length === 0 ? -1 : rowAtTime(rows, moment.at);
  return rows.findIndex(({ line }) => line.id === moment.segmentId);
}

/** The line a moment marks, and whether its highlight is still showing. */
export interface MomentMark {
  lineId: string;
  flashing: boolean;
}

/** Props of {@link MomentFocus}. */
export interface MomentFocusProps {
  moment: TranscriptMoment;
  rows: readonly TranscriptRow[];
  reveal: (row: number) => void;
  /** Marks the line, flashing, then again once the highlight has faded. */
  onMark: (mark: MomentMark) => void;
  /** Called once the moment has been shown (or cannot be), to drop it from the URL. */
  onDone: (() => void) | undefined;
}

/** Renders nothing; focuses the moment once. */
export function MomentFocus({ moment, rows, reveal, onMark, onDone }: MomentFocusProps) {
  useMountEffect(() => {
    const row = rowOfMoment(rows, moment);
    const line = rows[row]?.line;
    if (line === undefined) {
      onDone?.();
      return;
    }
    reveal(row);
    onMark({ lineId: line.id, flashing: true });
    const timer = window.setTimeout(() => {
      onMark({ lineId: line.id, flashing: false });
      onDone?.();
    }, MOMENT_FLASH_MS);
    return () => {
      window.clearTimeout(timer);
    };
  });
  return null;
}
