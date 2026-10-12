/** Every visible string of onboarding, in one place (§3A). */
import type { OnboardingStep, PermissionKind, PermissionState } from './types';

/** The short name of each step, for the progress indicator and Settings. */
export const stepNames: Record<OnboardingStep, string> = {
  welcome: 'Welcome',
  permissions: 'Permissions',
  ai_setup: 'AI setup',
  calendar: 'Calendar',
  profile: 'Profile',
  try_it: 'Try it',
};

export const flowCopy = {
  back: 'Back',
  skip: 'Skip',
  continue: 'Continue',
  skipSetup: 'Skip setup',
  progress: (index: number, total: number) => `Step ${String(index)} of ${String(total)}`,
  saveFailed: 'Your progress could not be saved.',
};

export const welcomeCopy = {
  title: 'Welcome to Deino',
  description:
    'Deino takes notes for your meetings. It listens on this Mac, transcribes on this Mac, and keeps everything here.',
  points: [
    'No meeting bot joins your calls.',
    'Transcription never leaves your Mac.',
    'Notes use the AI provider you choose, with your own key.',
  ],
  start: 'Get started',
};

export const permissionsCopy = {
  title: 'Permissions',
  description: 'Deino needs to hear both sides of a call. You can change any of these later.',
  optional: 'Optional',
  allow: 'Allow',
  openSettings: 'Open System Settings',
  check: 'Check again',
  states: {
    granted: 'Allowed',
    denied: 'Turned off',
    not_determined: 'Not asked yet',
    unknown: 'Not checked yet',
    unsupported: 'Not needed here',
  } satisfies Record<PermissionState, string>,
  kinds: {
    microphone: { name: 'Microphone', why: 'Your side of the conversation.' },
    system_audio: {
      name: 'System audio',
      why: 'Everyone else on the call, captured straight from your Mac.',
    },
    notifications: { name: 'Notifications', why: 'Reminders before meetings start.' },
    accessibility: {
      name: 'Accessibility',
      why: 'Speaker names from Zoom, Meet and Teams, and meeting links in your browser.',
    },
    screen_recording: {
      name: 'Screen Recording',
      why: 'Optional: snapshots and video of the meeting window, once you turn screen capture on.',
    },
  } satisfies Record<PermissionKind, { name: string; why: string }>,
  calendar: { name: 'Calendar', why: 'Titles and attendees for your meetings.' },
  missingRequired: 'Recording needs the microphone and system audio.',
};

/** Settings → Recording → Permissions (UX-10): the same rows as the step. */
export const permissionsSettingsCopy = {
  title: 'Permissions',
  description:
    'What macOS lets Deino use. Recording needs the microphone and system audio; the rest turn on features.',
  loadFailed: 'Couldn’t read the permissions.',
};

export const aiCopy = {
  title: 'AI setup',
  description:
    'Transcription runs on this Mac. Notes are written by the AI you pick: Claude or OpenAI with your own key, or a smaller model that runs here.',
  models: 'On this Mac',
  transcription: 'Transcription',
  transcriptionWhy: 'Parakeet, English speech to text.',
  embedding: 'Search',
  embeddingWhy: 'Finds meetings by meaning, not only by words.',
  download: 'Download',
  downloading: (percent: number) => `${String(percent)}%`,
  ready: 'Ready',
  retry: 'Try again',
  notesTitle: 'Notes',
  choices: { claude: 'Claude', openai: 'OpenAI', local: 'On this Mac' },
  keyLabel: (provider: string) => `${provider} API key`,
  keyHelp: 'Saved in your Keychain. Deino never shows it again.',
  check: 'Check and save',
  checking: 'Checking…',
  keySaved: 'Key saved',
  keyRejected: 'That key did not work. Check it and try again.',
  localWhy: 'Private and free, but slower and less thorough than Claude or OpenAI.',
};

export const calendarCopy = {
  title: 'Calendar',
  description:
    'Connect a calendar so meetings arrive with their titles and attendees, and Deino can remind you before they start.',
  more: 'A Google or Microsoft account that is not on this Mac can be connected later in Settings → Calendar.',
};

export const profileCopy = {
  title: 'About you',
  description: 'A few words about you help the AI write notes that sound like you.',
  saving: 'Saving…',
  saveFailed: 'Your profile could not be saved.',
  more: 'Writing style and background can be added later in Settings → Notes.',
};

export const tryItCopy = {
  title: 'Try it',
  description: 'Record a short test, or open a sample meeting to see what Deino makes of a call.',
  testName: 'Record a 20-second test',
  testWhy: 'Play a video or talk for a moment. It is transcribed here on your Mac.',
  start: 'Record',
  retry: 'Try again',
  stop: 'Stop now',
  remaining: (seconds: number) => `Recording · ${String(seconds)}s left`,
  recording: 'Starting…',
  saving: 'Finishing the transcript…',
  done: 'Saved as a meeting. Notes are on the way.',
  ready: 'Ready',
  open: 'Open it',
  finish: 'Finish',
  failed: 'The test could not start.',
  testTitle: 'Test recording',
  sampleName: 'Open the sample meeting',
  sampleWhy: 'A short standup with notes, a transcript and action items. Delete it any time.',
  sampleOpen: 'Open',
  sampleOpening: 'Opening…',
  sampleFailed: 'The sample could not be added.',
};

export const resumeCopy = {
  title: 'Setup',
  description: 'Go back through first-run setup: permissions, AI, calendar and profile.',
  allDone: 'Every step is done.',
  left: (names: string) => `Not done yet: ${names}.`,
  resume: 'Resume setup',
  restart: 'Run setup again',
};
