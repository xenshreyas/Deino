/** Types of the meeting player: the generated ones and what the dock draws. */
export type {
  DownloadOutcome,
  MediaFile,
  MediaSource,
  MeetingId,
  MeetingMedia,
} from '@/lib/ipc/generated/player';

/** The audio sources the player can switch between (video follows the audio). */
export type AudioSource = 'mixed' | 'microphone' | 'system';

/**
 * A stretch of the recording one speaker held, for the scrubber's activity
 * bands. `color` is the speaker's palette slot (`--speaker-1` …).
 */
export interface SpeakerBand {
  /** Milliseconds from the start of the recording. */
  start: number;
  /** Milliseconds from the start of the recording; not before `start`. */
  end: number;
  /** Palette slot 1–6. */
  color: number;
}

/** Whether the player is loading, can play, or could not load its file. */
export type PlayerStatus = 'loading' | 'ready' | 'error';
