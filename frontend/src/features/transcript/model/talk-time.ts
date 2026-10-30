/**
 * Who talked how much (the meeting page's context rail): each speaker's
 * spoken time, summed from their finished lines' lengths, and their share
 * of the meeting's speech in whole percent. The shares add up to exactly
 * 100 (largest remainder), so the rail never reads 99% or 101%.
 */
import type { SpeakerView, TranscriptRow } from '../types';

/** One speaker's talk time. */
export interface TalkShare {
  speaker: SpeakerView;
  /** Milliseconds spoken. */
  ms: number;
  /** Whole percent of all speech; the shares add up to 100. */
  percent: number;
}

/** Each speaker's spoken milliseconds, in order of their first line. */
function spokenTime(
  rows: readonly TranscriptRow[],
): Map<string, { speaker: SpeakerView; ms: number }> {
  const totals = new Map<string, { speaker: SpeakerView; ms: number }>();
  for (const { line, speaker } of rows) {
    const length = line.end - line.start;
    if (line.open || length <= 0) continue;
    const total = totals.get(speaker.key);
    if (total === undefined) totals.set(speaker.key, { speaker, ms: length });
    else total.ms += length;
  }
  return totals;
}

/** Whole percents of `values` that add up to 100, rounding the largest remainders up. */
export function wholePercents(values: readonly number[]): number[] {
  const sum = values.reduce((total, value) => total + value, 0);
  if (sum <= 0) return values.map(() => 0);
  const exact = values.map((value) => (value / sum) * 100);
  const floors = exact.map(Math.floor);
  let left = 100 - floors.reduce((total, value) => total + value, 0);
  const byRemainder = exact
    .map((value, index) => ({ index, remainder: value - Math.floor(value) }))
    .sort((a, b) => b.remainder - a.remainder);
  for (const { index } of byRemainder) {
    if (left <= 0) break;
    floors[index] = (floors[index] ?? 0) + 1;
    left -= 1;
  }
  return floors;
}

/** Every speaker who spoke, most talk time first (ties keep transcript order). */
export function talkShares(rows: readonly TranscriptRow[]): TalkShare[] {
  const totals = [...spokenTime(rows).values()];
  const percents = wholePercents(totals.map((total) => total.ms));
  return totals
    .map((total, index) => ({ ...total, percent: percents[index] ?? 0 }))
    .sort((a, b) => b.ms - a.ms);
}
