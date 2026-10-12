import type { CallApp, TagState } from './types';

/** Every visible string of the speaker-tags feature. */
export const speakerTagsCopy = {
  section: {
    title: 'Meeting apps',
    description:
      'While Deino records, it reads who the call window shows as speaking and names the transcript after them. Names stay on this Mac.',
  },
  apps: {
    zoom: { name: 'Zoom', where: 'The Zoom desktop app' },
    teams: { name: 'Microsoft Teams', where: 'The Teams desktop app' },
    meet: { name: 'Google Meet', where: 'Meet in Google Chrome, as the active tab of its window' },
  } satisfies Record<CallApp, { name: string; where: string }>,
  states: {
    enabled: 'Enabled',
    needsPermission: 'Needs permission',
    unsupported: 'Unsupported',
  } satisfies Record<TagState, string>,
  unsupportedHint:
    'Not available here, or this version of the app is not recognised yet. Speakers are named by voice.',
  openSettings: 'Open System Settings',
  openSettingsLabel: (app: string) => `Open System Settings to allow speaker tags for ${app}`,
  permissionHint:
    'Allow Deino under Privacy & Security → Accessibility. Deino never turns this on by itself; this page updates when you come back.',
  settingsFailed: 'Could not open System Settings. Open Privacy & Security → Accessibility there.',
  loadFailed: 'Could not read the speaker-tag status.',
} as const;
