/**
 * The transcript view (TRN-01): a meeting's transcript as chat bubbles
 * grouped into speaker turns (the user on the right in the accent tint,
 * everyone else on the left with their name in their colour), virtualised
 * for long meetings, searchable with ⌘F (`speaker:` filters, "n of N",
 * Enter and ⇧Enter), live while recording (follows the newest line, "Jump
 * to now"), and deep-linkable to a moment (`?t=<ms>`).
 *
 * | Export | Where |
 * |---|---|
 * | {@link TranscriptPanel} | the meeting page's Transcript tab, the record screen |
 * | {@link TranscriptView} | anything that already holds rows (presentational) |
 * | {@link TRANSCRIPT_ACTIONS} | the bubble actions registry: TRN-02, TRN-03, TRN-04, TRN-06 fill their entries |
 * | `onSpeakerClick` | CAP-04's speaker rename and merge popover |
 */
export { TRANSCRIPT_ACTIONS } from './actions';
export type {
  TranscriptAction,
  TranscriptActionContext,
  TranscriptActionId,
  TranscriptActionState,
} from './actions';
export { transcriptApi } from './api';
export { TranscriptPanel, type TranscriptPanelProps } from './components/transcript-panel';
export { TranscriptView, type TranscriptViewProps } from './components/transcript-view';
export { transcriptCopy } from './copy';
export { transcriptKeys, transcriptQuery } from './queries';
export type {
  SpeakerClickHandler,
  SpeakerView,
  Transcript,
  TranscriptLine,
  TranscriptRow,
  TranscriptSpeaker,
} from './types';
