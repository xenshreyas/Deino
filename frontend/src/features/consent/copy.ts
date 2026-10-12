import type { CallApp } from '@/features/speaker-tags';

/** Every visible string of the consent feature. */
export const consentCopy = {
  reminder: {
    title: 'Let people know',
    body: 'Deino is transcribing. Tell everyone on the call, and stop if anyone would rather you didn’t.',
    label: 'Consent reminder',
    dismiss: 'Got it',
    never: 'Don’t show again',
    saveFailed: 'Could not save the reminder setting',
  },
  settings: {
    reminderSection: 'Tell people',
    description:
      'Help the people on your calls know they are being transcribed. Nothing is sent anywhere else.',
    reminderTitle: 'Remind me to tell people',
    reminderOn: 'A small card under the recording pill reminds you to tell everyone.',
    reminderOff: 'No reminder is shown.',
    chatTitle: 'Automatic chat message',
    chatDescription:
      'Once Deino hears speech, it posts your message in the meeting chat, once per meeting. It never posts while you are typing or into a message you have started.',
    messageTitle: 'Message',
    messageHint: (max: number) => `Posted as written, up to ${max} characters.`,
    messageEmpty: 'Write a message before turning an app on.',
    permission: 'Accessibility',
    saveFailed: 'Couldn’t save the consent settings',
  },
  apps: {
    zoom: { name: 'Zoom', where: 'The meeting chat in the Zoom desktop app' },
    teams: { name: 'Microsoft Teams', where: 'The meeting chat in the Teams desktop app' },
    meet: { name: 'Google Meet', where: 'In-call messages, in Google Chrome' },
  } satisfies Record<CallApp, { name: string; where: string }>,
} as const;
