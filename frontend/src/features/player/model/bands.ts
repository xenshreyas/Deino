/**
 * The scrubber's speaker activity as one CSS gradient.
 *
 * A two-hour meeting has thousands of lines; drawing a band per line would
 * put thousands of nodes in the dock. Instead the recording is cut into
 * {@link BUCKETS} equal slices, each painted in the colour of whoever spoke
 * most of it (nobody: the track shows through), and runs of equal slices
 * become one gradient stop pair. Colours are `var(--speaker-n)`, so both
 * themes repaint without recomputing.
 */
import type { SpeakerBand } from '../types';

/** Slices across the scrubber: finer than a pixel column at the dock's widths. */
export const BUCKETS = 400;

/** The colour slot of each slice (0 = silence). */
export function bucketColors(bands: readonly SpeakerBand[], duration: number): number[] {
  const slices = new Array<number>(BUCKETS).fill(0);
  if (duration <= 0) return slices;
  const width = duration / BUCKETS;
  const spoken = Array.from({ length: BUCKETS }, () => new Map<number, number>());
  for (const band of bands) {
    const first = Math.max(0, Math.floor(band.start / width));
    const last = Math.min(BUCKETS - 1, Math.floor(Math.max(band.start, band.end - 1) / width));
    for (let slice = first; slice <= last; slice += 1) {
      const overlap = Math.min(band.end, (slice + 1) * width) - Math.max(band.start, slice * width);
      const tally = spoken[slice];
      if (overlap > 0 && tally !== undefined) {
        tally.set(band.color, (tally.get(band.color) ?? 0) + overlap);
      }
    }
  }
  return spoken.map(loudest);
}

/** The colour with the most speech in a slice, or 0. */
function loudest(tally: ReadonlyMap<number, number>): number {
  let best = 0;
  let most = 0;
  for (const [color, ms] of tally) {
    if (ms > most) [best, most] = [color, ms];
  }
  return best;
}

/** `linear-gradient(to right, …)` painting each slice's speaker. */
export function bandGradient(bands: readonly SpeakerBand[], duration: number): string {
  const slices = bucketColors(bands, duration);
  const stops: string[] = [];
  let from = 0;
  for (let slice = 1; slice <= BUCKETS; slice += 1) {
    if (slice < BUCKETS && slices[slice] === slices[from]) continue;
    const color = slices[from] ?? 0;
    const paint = color === 0 ? 'transparent' : `var(--speaker-${String(color)})`;
    stops.push(`${paint} ${percent(from)} ${percent(slice)}`);
    from = slice;
  }
  return `linear-gradient(to right, ${stops.join(', ')})`;
}

/** A slice boundary as a percentage of the track. */
function percent(slice: number): string {
  return `${String(Math.round((slice / BUCKETS) * 10_000) / 100)}%`;
}
