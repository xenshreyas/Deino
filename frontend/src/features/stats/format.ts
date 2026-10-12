/**
 * Number formatting for the Stats page: compact counts, hours, estimated
 * dollars and short dates. Pure, so tests pin every case.
 */

const SECONDS_PER_HOUR = 3600;
const MICROS_PER_DOLLAR = 1_000_000;

const compact = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 });
const whole = new Intl.NumberFormat('en');

/** A count, compacted past ten thousand: 1,284 · 12.9K · 4.2M. */
export function formatCount(value: number): string {
  return value < 10_000 ? whole.format(value) : compact.format(value);
}

/** Recorded seconds as hours: one decimal under ten hours, whole above. */
export function formatHours(seconds: number): string {
  const hours = seconds / SECONDS_PER_HOUR;
  if (hours === 0) return '0';
  if (hours < 10) return hours.toFixed(1);
  return formatCount(Math.round(hours));
}

/** Millionths of a dollar as dollars: $0 · <$0.01 · $0.42 · $12.50 · $1.2K. */
export function formatCost(micros: number): string {
  const dollars = micros / MICROS_PER_DOLLAR;
  if (micros === 0) return '$0';
  if (dollars < 0.01) return '<$0.01';
  if (dollars < 1_000) return `$${dollars.toFixed(2)}`;
  return `$${compact.format(dollars)}`;
}

/** A calendar day (`YYYY-MM-DD`) as the user reads it: "Tue, Sep 29, 2026". */
export function formatDay(day: string): string {
  const date = parseDay(day);
  return date.toLocaleDateString('en', {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    timeZone: 'UTC',
  });
}

/** A calendar day at UTC midnight, so arithmetic never crosses a zone. */
export function parseDay(day: string): Date {
  return new Date(`${day}T00:00:00Z`);
}

/** An hour of the day as a short tick: 12a, 6a, 12p, 6p. */
export function hourTick(hour: number): string {
  const suffix = hour < 12 ? 'a' : 'p';
  const twelve = hour % 12 === 0 ? 12 : hour % 12;
  return `${String(twelve)}${suffix}`;
}

/** An instant as a short date: "Dec 3, 2026". */
export function formatDate(instant: string): string {
  return new Date(instant).toLocaleDateString('en', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

/** `part` of `total` as a whole percentage; 0 when there is no total. */
export function percent(part: number, total: number): number {
  return total === 0 ? 0 : Math.round((part / total) * 100);
}
