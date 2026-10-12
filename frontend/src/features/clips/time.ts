/** Clip times as the UI writes them: `0:12`, and `0:12 – 0:27` for a range. */
import { formatElapsed } from '@/features/recording';

import { clipsCopy } from './copy';

/** `ms` milliseconds into the recording, as `m:ss` (or `h:mm:ss`). */
export function clipTime(ms: number): string {
  return formatElapsed(ms / 1000);
}

/** A clip's range, `0:12 – 0:27`. */
export function clipRange(start: number, end: number): string {
  return clipsCopy.range(clipTime(start), clipTime(end));
}
