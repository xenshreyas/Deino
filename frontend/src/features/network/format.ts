/** Formatting for the egress log. */

/** Bytes per kibibyte; sizes are shown in binary units, like Finder's "KB". */
const KIB = 1024;

/** Units after bytes, in order. */
const UNITS = ['KB', 'MB', 'GB', 'TB'] as const;

/** `0 B`, `512 B`, `2.0 KB`, `1.4 MB`, `731 MB`, `1.2 GB`. */
export function formatBytes(bytes: number): string {
  if (bytes < KIB) return `${String(Math.max(0, Math.round(bytes)))} B`;
  let value = bytes / KIB;
  let unit: (typeof UNITS)[number] = 'KB';
  for (const next of UNITS.slice(1)) {
    if (value < KIB) break;
    value /= KIB;
    unit = next;
  }
  const digits = value < 10 ? 1 : 0;
  return `${value.toFixed(digits)} ${unit}`;
}

/** A request's local time, 24-hour: `Oct 9, 09:30:05`. */
const TIME_FORMAT = new Intl.DateTimeFormat('en', {
  month: 'short',
  day: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
});

/** An ISO-8601 instant as the log shows it. */
export function formatTime(iso: string): string {
  return TIME_FORMAT.format(new Date(iso));
}
