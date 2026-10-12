/** Every user-facing string of the meeting player. */
import type { MediaSource } from './types';

export const playerCopy = {
  label: 'Recording',
  play: 'Play',
  pause: 'Pause',
  playShortcut: 'Space',
  back: 'Back 15 seconds',
  forward: 'Forward 15 seconds',
  backShortcut: '←',
  forwardShortcut: '→',
  scrubber: 'Playback position',
  position: (now: string, total: string) => `${now} of ${total}`,
  speed: 'Playback speed',
  rate: (rate: number) => `${String(rate)}×`,
  track: 'Audio track',
  showVideo: 'Show video',
  hideVideo: 'Hide video',
  download: 'Download',
  saved: 'Saved',
  saveFailed: 'The recording could not be saved',
  cannotPlay: 'This recording cannot be played',
} as const;

/** The track menu's names. */
export const sourceLabels: Readonly<Record<MediaSource, string>> = {
  mixed: 'Mixed',
  microphone: 'Me',
  system: 'Them',
  video: 'Video',
};

/** The download menu's items. */
export const downloadLabels: Readonly<Record<MediaSource, string>> = {
  mixed: 'Recording (mixed)',
  microphone: 'My audio',
  system: 'Their audio',
  video: 'Screen recording',
};
