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
  sections: {
    general: { label: 'General', description: 'Notifications, storage and how Deino starts.' },
    appearance: { label: 'Appearance', description: 'Theme for every Deino window.' },
    setup: {
      label: 'Setup',
      description: 'Go back through first-run setup: permissions, AI, calendar and profile.',
    },
    recording: {
      label: 'Recording',
      description: 'Where recordings are saved and how audio is captured.',
    },
    detection: {
      label: 'Meeting detection',
      description: 'Offer to take notes when a call starts, and which apps to ignore.',
    },
    transcription: {
      label: 'Transcription',
      description: 'Live or after the meeting, and the local speech-to-text engine and its models.',
    },
    vocabulary: {
      label: 'Vocabulary',
      description: 'Names and terms Deino should spell your way, and replacements.',
    },
    people: {
      label: 'People',
      description: 'Recognise speakers by voice, and the voice profiles Deino knows.',
    },
    ai: {
      label: 'AI models',
      description: 'Which provider and model serve each AI task, and a quick test for each.',
    },
    models: { label: 'Summary models', description: 'The language model that writes your notes.' },
    templates: {
      label: 'Templates',
      description: 'What your notes contain for each kind of meeting, and when each is used.',
    },
    profile: {
      label: 'Profile',
      description: 'Who you are and how you like notes written, for every AI prompt.',
    },
    search: {
      label: 'Search index',
      description: 'Find meetings by meaning with a small model that runs on this Mac.',
    },
    calendar: {
      label: 'Calendar',
      description: 'Your calendars from macOS Calendar, and which ones Deino shows.',
    },
    integrations: {
      label: 'Integrations',
      description: 'Send notes, tasks and messages to the tools you use.',
    },
    network: {
      label: 'Network',
      description: 'What may leave this Mac, what is hidden first, and a log of what left.',
    },
    export: {
      label: 'Export',
      description: 'Notes into a folder after every meeting, and every meeting at once.',
    },
    beta: { label: 'Beta', description: 'Features that are still being finished.' },
    about: { label: 'About', description: 'Version, privacy and updates.' },
    developer: {
      label: 'Developer',
      description: 'The deino command-line tool, and Claude Code access to your meetings.',
    },
  },
} as const;

/** The placeholder for an API key field: a typed key replaces a saved one. */
export function apiKeyPlaceholder(status: SecretStatus | undefined): string {
  return status === 'stored'
    ? settingsCopy.keyStoredPlaceholder
    : settingsCopy.keyMissingPlaceholder;
}
