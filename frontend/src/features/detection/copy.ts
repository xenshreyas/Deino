import type { MeetingKind } from './types';

/** Every visible string of the detection feature (Granola §2.3, Circleback §3.5). */
export const detectionCopy = {
  titles: {
    meeting: 'Meeting detected',
    huddle: 'Huddle detected',
    call: 'Call detected',
  } satisfies Record<MeetingKind, string>,
  takeNotes: 'Take notes',
  ignore: 'Ignore',
  dontRecord: 'Don’t record',
  alwaysIgnore: 'Always ignore this app',
  moreIgnore: 'More ways to ignore',
  promptLabel: 'Meeting prompt',
  countdown: (seconds: number) => `Recording in ${seconds}s`,
  failed: 'That didn’t work. Try again from the main window.',

  settings: {
    detectTitle: 'Detect meetings',
    detectDescription:
      'When another app starts using your microphone, offer to take notes. Nothing is recorded until you choose.',
    autoStartTitle: 'Start recording automatically',
    autoStartDescription: (seconds: number) =>
      `After a ${seconds}-second countdown, with a Don’t record button to cancel.`,
    countdownLabel: 'Countdown',
    countdownUnit: 'seconds',
    ignoredTitle: 'Ignored apps',
    ignoredDescription: 'These apps never prompt, even when they use the microphone.',
    ignoredEmptyTitle: 'No ignored apps',
    ignoredEmptyBody: 'Choose “Always ignore this app” on a prompt to add one here.',
    stopIgnoring: (app: string) => `Stop ignoring ${app}`,
    browserHint:
      'Allow Deino in System Settings → Privacy & Security → Accessibility to recognise Meet, Zoom and Teams calls in the browser within seconds.',
    unsupported: 'Meeting detection is available on macOS.',
  },
} as const;
