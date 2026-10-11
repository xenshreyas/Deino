import type { CalendarAccess, CalendarProvider, ConferencePlatform } from './types';

/** Every visible string of the calendar feature. */
export const calendarCopy = {
  comingUp: 'Coming up',
  today: 'Today',
  tomorrow: 'Tomorrow',
  allDay: 'All day',
  now: 'Now',
  inMinutes: (minutes: number) => `in ${minutes}m`,
  people: (count: number) => (count === 1 ? '1 person' : `${count} people`),
  external: 'External',
  loadFailed: 'Could not load your calendar.',
  nothingComingUp: 'Nothing coming up',
  nothingComingUpBody: 'No meetings in the next two weeks.',
  connectTitle: 'Connect your calendar',
  connectBody: 'See your next meetings here. Deino reads macOS Calendar, on this Mac only.',
  connectAction: 'Set up calendars',
  untitled: 'Untitled event',

  platforms: {
    zoom: 'Zoom',
    google_meet: 'Google Meet',
    teams: 'Teams',
    webex: 'Webex',
    slack: 'Slack huddle',
    other: 'Video call',
  } satisfies Record<ConferencePlatform, string>,

  sync: {
    title: 'macOS Calendar',
    description:
      'Every account on this Mac (iCloud, Google, Exchange) with no sign-in. Your events never leave the Mac.',
    label: 'Sync calendars',
    access: {
      not_determined: 'macOS will ask for permission to read your calendars.',
      granted: 'Deino reads your calendars to show what is coming up.',
      denied:
        'Calendar access is off for Deino. Turn it on in System Settings, then switch sync on again.',
      restricted: 'A device policy does not allow calendar access on this Mac.',
      write_only: 'Deino may add events but not read them. Allow full access in System Settings.',
      unsupported: 'Calendar sync needs macOS.',
    } satisfies Record<CalendarAccess, string>,
    openSettings: 'Open System Settings',
    neverSynced: 'Not synced yet',
    justNow: 'just now',
    synced: (when: string) => `Synced ${when}`,
    refresh: 'Refresh',
    refreshing: 'Syncing…',
    switchFailed: 'Could not change calendar sync.',
    settingsFailed: 'Could not open System Settings. Open Privacy & Security → Calendars there.',
  },

  calendars: {
    title: 'Calendars',
    description: 'Choose which calendars Deino shows. Hidden calendars keep syncing.',
    empty: 'No calendars yet. Turn on sync to see them here.',
    loadFailed: 'Could not load your calendars.',
    toggleFailed: 'Could not show or hide that calendar.',
    show: (title: string) => `Show ${title}`,
    untitled: 'Untitled calendar',
  },

  connectors: {
    title: 'Direct connections',
    description:
      'Optional, for an account that is not on this Mac. Uses your own app registration; events go straight from the provider to this Mac.',
    names: {
      apple: 'macOS Calendar',
      google: 'Google Calendar',
      microsoft: 'Microsoft Outlook',
    } satisfies Record<CalendarProvider, string>,
    setup: {
      apple: '',
      google: 'Create a Desktop app OAuth client in Google Cloud, then paste its ID and secret.',
      microsoft: 'Register a public client app in Microsoft Entra, then paste its client ID.',
    } satisfies Record<CalendarProvider, string>,
    clientId: 'Client ID',
    clientIdPlaceholder: 'Paste the client ID',
    clientSecret: 'Client secret',
    secretStored: 'Saved in the Keychain',
    secretMissing: 'Paste the client secret',
    needsSetup: 'Add your app registration to connect.',
    notConnected: 'Not connected',
    connected: (synced: string) => `Connected · ${synced}`,
    connect: 'Connect',
    waiting: 'Finish signing in in your browser…',
    disconnect: 'Disconnect',
    connectFailed: 'Could not connect. Check the app registration and try again.',
    disconnectFailed: 'Could not disconnect.',
    saveFailed: 'Could not save the app registration.',
  },
} as const;
