/**
 * Types of the transcript view: the generated ones, and the lines, speakers
 * and rows it draws.
 */
import type {
  Channel,
  MeetingId,
  PaletteColor,
  SegmentId,
  SpeakerId,
} from '@/lib/ipc/generated/transcript';

export type {
  Channel,
  MeetingId,
  PaletteColor,
  Segment,
  SegmentId,
  SpeakerId,
  Transcript,
  TranscriptSpeaker,
} from '@/lib/ipc/generated/transcript';
export type { SegmentUpdate } from '@/lib/ipc/generated/recording';

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
  /** Still being spoken: the text may grow. */
  open: boolean;
}

/** Who said a line, as the view draws them. */
export interface SpeakerView {
  /** Groups lines into turns: the speaker's ID, or a channel fallback. */
  key: string;
  /** The stored speaker, when the line names one. */
  speakerId: SpeakerId | null;
  name: string;
  /** The user's own voice: bubbles on the right, in the accent tint. */
  isMe: boolean;
  /** Palette slot 1–6 (`--speaker-1` …). */
  color: PaletteColor;
  /** Known to be someone (the user, or a person); unknown voices are muted. */
  identified: boolean;
}

/** One row of the list: a bubble, the first of its turn carrying the header. */
export interface TranscriptRow {
  line: TranscriptLine;
  speaker: SpeakerView;
  /** The first bubble of a speaker turn: it shows the name and time. */
  turnStart: boolean;
}

/** A character range of a line's text. */
export interface TextRange {
  start: number;
  end: number;
}

/** Called when the user clicks a speaker's name (CAP-04 opens its rename popover). */
export type SpeakerClickHandler = (speaker: SpeakerView, anchor: HTMLElement) => void;

/** The meeting a view shows, for actions that act on it. */
export interface TranscriptScope {
  meetingId: MeetingId;
}
