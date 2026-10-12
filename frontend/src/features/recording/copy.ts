/** Every user-facing string of the recording feature. */
import type { HealthIssue, StopReason } from './types';

export const recordingCopy = {
  start: 'Start recording',
  starting: 'Starting…',
  pause: 'Pause',
  resume: 'Resume',
  stop: 'Stop',
  stopping: 'Saving…',
  paused: 'Paused',
  untitled: 'New recording',
  idleTitle: 'Ready to record',
  idleBody: 'Start recording to take notes and get a live transcript.',
  startFailed: 'Recording could not start',
  stopFailed: 'Recording could not stop',
  pauseFailed: 'Recording could not pause',
  switchFailed: 'The recording could not switch devices',
  resumeMeeting: 'Resume recording',
  resumeMeetingHint: 'Record more of this meeting; new audio is added at the end',
  resumeFailed: 'Recording could not resume',
  openSettings: 'Open settings',
  health: {
    recovering: 'Audio interrupted; reconnecting…',
    recovered: 'Audio is back',
    deviceLost: 'An audio device was disconnected; that side records silence',
    framesDropped: 'A moment of audio was lost',
    storageStalled: 'Saving audio is slow',
    storageFailed: 'Audio could not be saved',
    diskLow: 'Your disk is nearly full',
    bluetoothCallMode: 'Your Bluetooth headset is in call mode; audio quality drops',
    systemAudioUnavailable: 'System audio is unavailable; only your microphone is recorded',
    asleep: 'Your Mac went to sleep; recording paused',
    awake: 'Your Mac woke; recording continues',
  },
} as const;

/** The record panel's strings (CAP-10, CAP-07). */
export const panelCopy = {
  label: 'Recording',
  openNotes: 'Open notes',
  openNotesHint: 'Show this meeting’s notes',
  pause: 'Pause recording',
  resume: 'Resume recording',
  stop: 'Stop recording',
  more: 'More',
  microphone: 'Microphone',
  screen: 'Screen to capture',
  systemDefault: 'System default',
  recordCallAudio: 'Record the call’s audio',
  keepRecording: 'Keep recording',
  ending: (seconds: number) => `Ending in ${String(seconds)}s`,
  splitTitle: (title: string) => `${title} has started`,
  splitBody: 'Start a new note for it?',
  splitAccept: 'New note',
  splitDismiss: 'Not now',
  untitledEvent: 'The next meeting',
  starting: 'Starting',
  saving: 'Saving',
} as const;

/** Why the recording is ending, in a few words. */
export const endingReason: Record<StopReason, string> = {
  micReleased: 'The call ended',
  noSpeech: 'No one has spoken for 15 minutes',
  calendarEnded: 'The meeting is over',
  sleep: 'Your Mac is going to sleep',
  diskFull: 'Your disk is full',
};

/** A health issue, short enough for the pill. */
export function issueLabel(issue: HealthIssue): string {
  switch (issue.kind) {
    case 'noAudio':
      return issue.channel === 'system' ? 'No call audio' : 'No mic audio';
    case 'noTranscript':
      return 'Not transcribing';
    case 'transcriptionStopped':
      return 'Transcription restarting';
    case 'diskLow':
      return 'Disk nearly full';
    case 'diskFull':
      return 'Disk full';
  }
}
