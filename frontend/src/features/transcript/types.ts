/**
 * Types of the transcript view: the generated ones, and the lines, speakers
 * and rows it draws.
 */
import type { Channel, SegmentId, SpeakerId, WordTiming } from '@/lib/ipc/generated/transcript';

export type {
  Channel,
  MeetingId,
  MeetingMatches,
  ReplaceOutcome,
  ReplacePreview,
  ReplaceRequest,
  ReplaceScope,
  Segment,
  SegmentId,
  SpeakerId,
  Transcript,
  WordTiming,
} from '@/lib/ipc/generated/transcript';
export type { SegmentUpdate } from '@/lib/ipc/generated/recording';
export type { Participant } from '@/features/speakers';

/** One bubble: a saved segment, or a live line still being spoken. */
export interface TranscriptLine {
  id: SegmentId;
  /** Me (`mic`), Them (`system`) or both (`mixed`, imports). */
  channel: Channel;
  /** Who said it, once the recorder, diarization or the user knows. */
  speakerId: SpeakerId | null;
  /** Milliseconds from the start of the recording. */
  start: number;
  /** Milliseconds from the start of the recording; `start` while still open. */
  end: number;
  text: string;
  /** When each word was said, if the engine said; empty for live lines. */
  words: readonly WordTiming[];
  /** When the user last edited the text (TRN-02), if ever. */
  editedAt: string | null;
  /** Still being spoken: the text may grow. */
  open: boolean;
}

/** Who said a line, as the view draws them (from the meeting's participants). */
export interface SpeakerView {
  /** Groups lines into turns: the speaker's ID, or a channel fallback. */
  key: string;
  /** The stored speaker, when the line names one; only these can be renamed. */
  speakerId: SpeakerId | null;
  name: string;
  /** The user's own voice: bubbles on the right, in the accent tint. */
  isMe: boolean;
  /** Palette slot 1–6 (`--speaker-1` …). */
  color: number;
  /** Nobody is assigned to the voice yet: muted, with a dashed avatar. */
  unnamed: boolean;
}

/** One row of the list: a bubble, the first of its turn carrying the header. */
export interface TranscriptRow {
  line: TranscriptLine;
  speaker: SpeakerView;
  /** The first bubble of a speaker turn: it shows the name and time. */
  turnStart: boolean;
}

/**
 * A moment a link asks the view to show: a time of the recording (`?t=`),
 * or one line (`?segment=`, a citation).
 */
export type TranscriptMoment =
  | { kind: 'time'; /** Milliseconds from the start of the recording. */ at: number }
  | { kind: 'line'; segmentId: string };

/** A character range of a line's text. */
export interface TextRange {
  start: number;
  end: number;
}
