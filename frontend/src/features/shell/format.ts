/** Formatting helpers of the shell. */

const SECONDS_PER_MINUTE = 60;
const SECONDS_PER_HOUR = 3600;

/** `12:04`, or `1:02:03` past an hour; `0:00` for a missing or negative time. */
export function formatElapsed(seconds: number | null): string {
  const whole = Math.max(0, Math.floor(seconds ?? 0));
  const hours = Math.floor(whole / SECONDS_PER_HOUR);
  const minutes = Math.floor((whole % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE);
  const rest = String(whole % SECONDS_PER_MINUTE).padStart(2, '0');
  if (hours === 0) return `${minutes}:${rest}`;
  return `${hours}:${String(minutes).padStart(2, '0')}:${rest}`;
}
