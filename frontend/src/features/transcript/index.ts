/**
 * The transcript view (TRN-01): a meeting's transcript as chat bubbles
 * grouped into speaker turns (the user on the right in the accent tint,
 * everyone else on the left with their name in their colour, named from
 * the participants and renamed with CAP-04's "Who is …?"), virtualised
 * for long meetings, searchable with ⌘F (`speaker:` filters, "n of N",
 * Enter and ⇧Enter), live while recording (follows the newest line, "Jump
 * to now"), and deep-linkable to a moment (`?t=<ms>`). With a recording in
 * the player (TRN-03) it lights the line and word being played, follows
 * them, and seeks where it is clicked; its lines can be edited in place,
 * deleted with Undo, and found and replaced across meetings (TRN-02).
 *
 * | Export | Where |
 * |---|---|
 * | {@link TranscriptPanel} | the meeting page's Transcript tab, the record screen |
 * | {@link TranscriptView} | anything that already holds rows (presentational) |
 * | {@link TRANSCRIPT_ACTIONS} | the bubble actions registry: TRN-04 and TRN-06 fill their entries |
 * | {@link useSpeakerBands} | the player's scrubber: who spoke when |
 * | {@link useTalkTime} | the meeting page's context rail: who talked how much |
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
export { useSpeakerBands } from './hooks/use-speaker-bands';
export { useTalkTime } from './hooks/use-talk-time';
export type { TalkShare } from './model/talk-time';
export { timedWords } from './model/word-ranges';
export { transcriptKeys, transcriptQuery } from './queries';
export type {
  SpeakerView,
  Transcript,
  TranscriptLine,
  TranscriptMoment,
  TranscriptRow,
} from './types';
