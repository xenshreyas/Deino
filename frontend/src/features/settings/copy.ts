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
      description: 'The local speech-to-text engine and its models.',
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
      description: 'What may leave this Mac, and a log of everything that did.',
    },
    export: {
      label: 'Export',
      description: 'Notes into a folder after every meeting, and every meeting at once.',
    },
    beta: { label: 'Beta', description: 'Features that are still being finished.' },
    about: { label: 'About', description: 'Version, privacy and updates.' },
    developer: { label: 'Developer', description: 'Tools for development builds only.' },
  },
} as const;

/** The placeholder for an API key field: a typed key replaces a saved one. */
export function apiKeyPlaceholder(status: SecretStatus | undefined): string {
  return status === 'stored'
    ? settingsCopy.keyStoredPlaceholder
    : settingsCopy.keyMissingPlaceholder;
}
