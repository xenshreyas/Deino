import type { CallApp } from './types';

/** Every visible string of the speaker-tags feature. */
export const speakerTagsCopy = {
  section: {
    title: 'Speaker names from call apps',
    description:
      'While Deino records, it reads who the call window shows as speaking and names the transcript after them. Names stay on this Mac.',
  },
  apps: {
    zoom: { name: 'Zoom', where: 'The Zoom desktop app' },
    teams: { name: 'Microsoft Teams', where: 'The Teams desktop app' },
    meet: { name: 'Google Meet', where: 'Meet in Chrome, Brave, Edge or Arc, as the active tab of its window' },
  } satisfies Record<CallApp, { name: string; where: string }>,
  /** The permission, as the Permissions card names it (UX-09, UX-10: one row, there). */
  permission: 'Accessibility',
  /** The apps, as plain text. */
  worksWith: (apps: string) => `Works with ${apps}.`,
  unsupportedApp: (app: string) => `${app} (not supported here)`,
  unavailable: 'Not available on this Mac. Speakers are named by voice.',
  loadFailed: 'Couldn’t read the speaker-name status.',
} as const;
