/**
 * The retention periods the settings offer, and the rule Rust enforces: a
 * transcript is kept at least as long as its recording
 * (`RetentionSettings::validate`).
 */
import type { RetentionPeriod, RetentionSettings } from './types';

/** Every period, shortest last ("never" keeps forever). */
export const PERIODS = [
  'never',
  'days7',
  'days30',
  'days90',
  'days365',
] as const satisfies readonly RetentionPeriod[];

const DAYS: Record<RetentionPeriod, number | null> = {
  never: null,
  days7: 7,
  days30: 30,
  days90: 90,
  days365: 365,
};

/** The number of days, or `null` for "never". */
export function periodDays(period: RetentionPeriod): number | null {
  return DAYS[period];
}

/** True when `period` keeps things at least as long as `other`. */
export function keepsAtLeast(period: RetentionPeriod, other: RetentionPeriod): boolean {
  const mine = DAYS[period];
  const theirs = DAYS[other];
  if (mine === null) return true;
  if (theirs === null) return false;
  return mine >= theirs;
}

/**
 * The change for keeping recordings for `audio`: the transcript rises with
 * it when it would otherwise be kept for less time.
 */
export function audioChange(
  current: RetentionSettings,
  audio: RetentionPeriod,
): Partial<RetentionSettings> {
  return keepsAtLeast(current.transcript, audio) ? { audio } : { audio, transcript: audio };
}
