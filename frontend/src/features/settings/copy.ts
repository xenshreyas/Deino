import type { SecretStatus } from './types';

/** Every visible string of the settings feature. */
export const settingsCopy = {
  keyStored: 'Saved in the Keychain. Type a new key to replace it.',
  keyStoredPlaceholder: 'Saved in the Keychain',
  keyMissingPlaceholder: 'Enter your API key',
  keyLocked:
    'macOS no longer lets Deino read the saved key, which happens after an app update. Enter the key again to fix it.',

  title: 'Settings',
  navLabel: 'Settings pages',
  saveFailed: 'Couldn’t save the setting',

  /** Settings → Network: secrets hidden from notes and exports (NOT-09). */
  redaction: {
    title: 'Sensitive information',
    description: 'Secrets said in a meeting, kept out of what Deino writes and shares.',
    switchTitle: 'Hide sensitive information',
    on: 'Passwords, API keys, card numbers, Social Security numbers and one-time codes become [redacted] before notes are written, and in every export and the deino command. The transcript keeps them until you delete them there.',
    off: 'Notes and exports include everything that was said.',
    saveFailed: 'Couldn’t save the privacy setting',
  },

  /** Settings → Transcription: when the transcript is written (CAP-08). */
  captureModes: {
    title: 'While you record',
    description: 'When the transcript is written, and how carefully.',
    liveTitle: 'Live transcription',
    liveOn: 'The transcript appears as people speak.',
    liveOff:
      'Off: only audio is recorded, which keeps the Mac cool on long calls. The transcript is written right after the meeting.',
    repassTitle: 'Re-transcribe after the meeting for better accuracy',
    repassOn:
      'After each meeting, the whole recording is transcribed again with full context and replaces the live transcript. Speakers are kept.',
    repassOff: 'Not needed: with live transcription off, every transcript already gets this pass.',
    saveFailed: 'Could not save the transcription setting',
  },
  /** Settings → Transcription: the engine recordings use, as saved. */
  engine: {
    label: 'Transcription engine',
    advanced: 'Advanced',
    loading: 'Loading the transcription settings…',
    loadFailed: 'The transcription settings could not be loaded.',
    names: {
      parakeet_unified: 'Parakeet Unified EN (Recommended, live)',
      parakeet: 'Parakeet TDT v3 (ONNX)',
      whisper: 'Whisper',
    },
  },
} as const;

/** Settings → Recording: the microphone, the record panel and auto-stop (CAP-10, CAP-07). */
export const recordingSettingsCopy = {
  heading: 'Recording',
  headingDescription: 'Recordings are always saved, and deleted only as Privacy says.',
  microphoneTitle: 'Microphone',
  microphoneDescription: 'Your side of the call. The record panel can switch it mid-call.',
  microphoneLocked: 'Locked while recording. Use the record panel to switch.',
  microphoneDefault: 'The Mac’s default',
  panelTitle: 'Show the record panel',
  panelDescription:
    'A small pill at the edge of the screen with the time, a level meter and the controls. It never takes focus from your call.',
  autoStopSection: 'End automatically',
  autoStopSectionDescription: 'Stop a recording when the call is over, without a click.',
  autoStopMore: 'More ways to end',
  autoStopTitle: 'End recordings automatically',
  autoStopDescription:
    'When the call app lets go of the microphone, after 15 minutes without speech, or when your Mac sleeps. You get 15 seconds to keep recording.',
  calendarTitle: 'Also when the calendar event is over',
  calendarDescription:
    'Ten minutes after the event ends, once nobody has spoken for two minutes. Off by default: meetings often run over.',
  nextMeetingTitle: 'Offer a new note when the next meeting starts',
  nextMeetingDescription:
    'If you are still recording when a meeting with other people begins, ask whether to start a new note for it.',
} as const;

/** Settings → General → Notifications (UX-17). */
export const notificationsCopy = {
  title: 'Notifications',
  description: 'Each kind of notification Deino sends, in one place.',
  recordingTitle: 'Recording started and stopped',
  recordingDescription: 'A notification when a recording starts and when it ends.',
  remindersTitle: 'Meeting reminders',
  remindersDescription:
    'A small window a minute before each meeting with two or more people, with Join and record.',
  actionItemsTitle: 'Action items due today',
  actionItemsDescription: 'One notification at 9:00 for what is due that day.',
  soundTitle: 'Play a sound',
  soundDescription: 'A short chime when a meeting prompt or reminder slides in.',
  moreLabel: 'Which calendars remind',
} as const;

/** Settings → Advanced → Notes after meetings (UX-11). */
export const autoNotesCopy = {
  title: 'Notes after meetings',
  description: 'Whether Deino writes notes on its own once a recording stops.',
  switchTitle: 'Write notes after each meeting',
  on: 'Notes are written as soon as the transcript is ready.',
  off: 'Off: notes are written only when you choose Enhance.',
} as const;

/** Settings → Advanced → Transcription: the section around the inherited panels. */
export const transcriptionSectionCopy = {
  title: 'Transcription',
  description: 'Live or after the meeting, and the speech-to-text engine on this Mac.',
} as const;

/** The one-line link a feature shows while a permission it needs is missing (UX-10). */
export const permissionLinkCopy = {
  needs: (permission: string) => `Needs ${permission}`,
  accessibility: 'Accessibility',
} as const;

/** The placeholder for an API key field: a typed key replaces a saved one. */
export function apiKeyPlaceholder(status: SecretStatus | undefined): string {
  return status === 'stored'
    ? settingsCopy.keyStoredPlaceholder
    : settingsCopy.keyMissingPlaceholder;
}
