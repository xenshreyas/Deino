/**
 * A deep link's moment (`?t=<ms>`, ORG-08 search results, NOT-03 "Show in
 * transcript"): on mount, scrolls the line being said at that time into
 * the middle of the view and marks it for a moment. Render it with
 * `key={at}`, so each new moment remounts it and only it.
 */
import { useMountEffect } from '@/hooks/useMountEffect';

import { rowAtTime } from '../model/rows';
import type { TranscriptRow } from '../types';

/** How long the moment stays marked before it fades back. */
export const MOMENT_FLASH_MS = 1_600;

/** Props of {@link MomentFocus}. */
export interface MomentFocusProps {
  /** Milliseconds from the start of the recording. */
  at: number;
  rows: readonly TranscriptRow[];
  reveal: (row: number) => void;
  /** Marks the line (its ID), then `null` when the moment has passed. */
  onFlash: (lineId: string | null) => void;
  /** Called once the moment has been shown, for example to drop `?t` from the URL. */
  onDone: (() => void) | undefined;
}

/** Renders nothing; focuses the moment once. */
export function MomentFocus({ at, rows, reveal, onFlash, onDone }: MomentFocusProps) {
  useMountEffect(() => {
    const row = rowAtTime(rows, at);
    const line = rows[row]?.line;
    if (line === undefined) return;
    reveal(row);
    onFlash(line.id);
    const timer = window.setTimeout(() => {
      onFlash(null);
      onDone?.();
    }, MOMENT_FLASH_MS);
    return () => {
      window.clearTimeout(timer);
    };
  });
  return null;
}
