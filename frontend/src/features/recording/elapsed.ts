/** Recording time: what the timer in the sidebar and the controls shows. */
import type { RecordingSnapshot } from './types';

const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;

/**
 * Seconds recorded at `now` (epoch milliseconds): the time since capture
 * started, less every pause, frozen while paused. Zero before it starts.
 */
export function elapsedSeconds(snapshot: RecordingSnapshot, now: number): number {
  if (snapshot.startedAt === null) return 0;
  const end = snapshot.pausedAt === null ? now : Date.parse(snapshot.pausedAt);
  const recorded = end - Date.parse(snapshot.startedAt) - snapshot.pausedMs;
  return Math.max(0, Math.floor(recorded / 1000));
}

/** `12:04`, or `1:02:03` past an hour. */
export function formatElapsed(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(whole / SECONDS_PER_HOUR);
  const minutes = Math.floor((whole % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
  const rest = String(whole % SECONDS_PER_MINUTE).padStart(2, '0');
  if (hours === 0) return `${minutes}:${rest}`;
  return `${hours}:${String(minutes).padStart(2, '0')}:${rest}`;
}
