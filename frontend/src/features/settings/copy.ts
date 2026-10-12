import type { SecretStatus } from './types';

/** Every visible string of the settings feature. */
export const settingsCopy = {
  keyStored: 'Saved in the Keychain. Type a new key to replace it.',
  keyStoredPlaceholder: 'Saved in the Keychain',
  keyMissingPlaceholder: 'Enter your API key',
  keyLocked:
    'macOS no longer lets Deino read the saved key, which happens after an app update. Enter the key again to fix it.',

  title: 'Settings',
  navLabel: 'Settings sections',

  /** Settings → Network: secrets hidden from notes and exports (NOT-09). */
  redaction: {
    title: 'Sensitive information',
    description: 'Secrets said in a meeting, kept out of what Deino writes and shares.',
    switchTitle: 'Hide sensitive information',
    on: 'Passwords, API keys, card numbers, Social Security numbers and one-time codes become [redacted] before notes are written, and in every export and the deino command. The transcript keeps them until you delete them there.',
    off: 'Notes and exports include everything that was said.',
    saveFailed: 'Could not save the privacy setting',
  },

  /** Settings → Transcription: when the transcript is written (CAP-08). */
  captureModes: {
    title: 'While you record',
    description: 'When the transcript is written, and how carefully.',
    liveTitle: 'Live transcription',
    liveOn: 'The transcript appears as people speak.',
    liveOff:
      'Off: only audio is recorded, which keeps the Mac cool on long calls. The transcript is written right after the meeting.',
    repassTitle: 'High-accuracy re-pass',
    repassOn:
      'After each meeting, the whole recording is transcribed again with full context and replaces the live transcript. Speakers are kept.',
    repassOff: 'Not needed: with live transcription off, every transcript already gets this pass.',
    saveFailed: 'Could not save the transcription setting',
  },
  groups: {
    app: 'App',
    capture: 'Capture',
    ai: 'AI',
    workspace: 'Workspace',
    privacy: 'Privacy',
    data: 'Data',
    more: 'More',
  },
} as const;

/** Settings → Recording: the record panel and auto-stop (CAP-10, CAP-07). */
export const recordingSettingsCopy = {
  heading: 'While recording',
  panelTitle: 'Show the record panel',
  panelDescription:
    'A small pill at the edge of the screen with the time, a level meter and the controls. It never takes focus from your call.',
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
