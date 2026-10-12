import type { CallApp, TagState } from './types';

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
    meet: { name: 'Google Meet', where: 'Meet in Google Chrome, as the active tab of its window' },
  } satisfies Record<CallApp, { name: string; where: string }>,
  /** The one Accessibility row (UX-09): one permission, one row. */
  permission: {
    name: 'Accessibility',
    why: 'Needed to read speaker names from Zoom, Teams and Meet.',
  },
  states: {
    enabled: 'Allowed',
    needsPermission: 'Needs permission',
    unsupported: 'Not available here',
  } satisfies Record<TagState, string>,
  /** The apps, as plain text under the row. */
  worksWith: (apps: string) => `Works with ${apps}.`,
  unsupportedApp: (app: string) => `${app} (not supported here)`,
  openSettings: 'Open System Settings',
  openSettingsLabel: 'Open System Settings to allow Accessibility',
  permissionHint:
    'Allow Deino under Privacy & Security → Accessibility. Deino never turns this on by itself; this page updates when you come back.',
  settingsFailed: 'Couldn’t open System Settings. Open Privacy & Security → Accessibility there.',
  loadFailed: 'Couldn’t read the speaker-name status.',
} as const;
