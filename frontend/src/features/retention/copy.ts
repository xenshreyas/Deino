import { periodDays } from './periods';
import type { RetentionPeriod } from './types';

/** "7 days", or "Never" (keep forever). */
function periodLabel(period: RetentionPeriod): string {
  const days = periodDays(period);
  return days === null ? 'Never' : `${String(days)} days`;
}

/** "1 October 2026". */
function day(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
}

/** Every visible string of the retention feature (PRV-01). */
export const retentionCopy = {
  settings: {
    title: 'Retention',
    description:
      'Delete old meetings, or parts of them, automatically. Deino checks once a day and counts from the day of the meeting.',
    period: periodLabel,
    audioTitle: 'Delete recordings after',
    audioDescription: 'Audio and screen recordings, clips included. Notes and transcripts stay.',
    transcriptTitle: 'Delete transcripts after',
    transcriptDescription:
      'Kept at least as long as recordings, so a recording is never left without its transcript. Notes stay, but cannot be regenerated.',
    transcriptRaised: 'Transcripts are now kept as long as recordings.',
    everythingTitle: 'Delete meetings after',
    everythingDescription: 'Whole meetings move to the trash, which empties itself after 30 days.',
    saveFailed: 'Could not save the retention setting',
  },

  deleted: {
    transcriptTitle: 'Transcript deleted',
    transcriptBody: (at: string) =>
      `Your retention settings deleted this transcript on ${day(at)}. The notes were kept.`,
    regenerateBlocked: 'The transcript was deleted, so these notes cannot be regenerated.',
  },

  tip: {
    title: 'Pause for sensitive discussions',
    body: 'Nothing is recorded or transcribed while the recording is paused.',
    dismiss: 'Got it',
  },
} as const;
