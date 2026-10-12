/**
 * Retention (PRV-01): recordings, transcripts and whole meetings deleted a
 * chosen number of days after the meeting, by a daily sweep in Rust.
 *
 * | Export | Used by |
 * |---|---|
 * | {@link RetentionSettings} | Settings → Privacy & security |
 * | {@link TranscriptGate} | the meeting's Transcript tab ("Transcript deleted") |
 * | {@link useRegenerateBlock} | everything that regenerates notes, disabled once the transcript is gone |
 * | {@link PauseTipSlot} | the record panel, under the pill |
 */
export { retentionApi } from './api';
export { PauseTip, type PauseTipProps, PauseTipSlot } from './components/pause-tip';
export { PeriodSelect, RetentionSettings } from './components/retention-settings';
export {
  TranscriptDeleted,
  TranscriptGate,
  type TranscriptGateProps,
} from './components/transcript-deleted';
export { retentionCopy } from './copy';
export { audioChange, keepsAtLeast, PERIODS, periodDays } from './periods';
export {
  meetingRetentionQuery,
  retentionKeys,
  useMeetingRetention,
  useRegenerateBlock,
} from './queries';
export type { MeetingRetention, RetentionPeriod, RetentionSettings as Retention } from './types';
