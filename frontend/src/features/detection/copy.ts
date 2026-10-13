import type { Detection, MeetingKind } from './types';

/** The services Deino recognises, by name; `other` falls back to the app. */
const SERVICES: Record<Exclude<Detection['platform'], 'other'>, string> = {
  zoom: 'Zoom',
  teams: 'Teams',
  googleMeet: 'Google Meet',
  webex: 'Webex',
  slack: 'Slack',
  faceTime: 'FaceTime',
  phone: 'Phone',
  whatsApp: 'WhatsApp',
  discord: 'Discord',
};

/** "17:30" in the user's own clock format. */
function clock(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
}

/** Every visible string of the detection feature (Granola §2.3, Circleback §3.5). */
export const detectionCopy = {
  titles: {
    meeting: 'Meeting detected',
    huddle: 'Huddle detected',
    call: 'Call detected',
  } satisfies Record<MeetingKind, string>,
  /** The service: "Google Meet", or the app when the service is unknown. */
  service: (detection: Detection) =>
    detection.platform === 'other' ? detection.app.name : SERVICES[detection.platform],
  /** Under the title: the event's start and service, or the app. */
  subtitle: (detection: Detection) =>
    detection.event === null
      ? detection.app.name
      : `${clock(detection.event.startsAt)} · ${detectionCopy.service(detection)}`,
  alwaysIgnoreApp: (app: string) => `Always ignore ${app}`,
  takeNotes: 'Take notes',
  ignore: 'Ignore',
  dontRecord: 'Don’t record',
  alwaysIgnore: 'Always ignore this app',
  moreIgnore: 'More ways to ignore',
  promptLabel: 'Meeting prompt',
  countdown: (seconds: number) => `Recording in ${seconds}s`,
  failed: 'That didn’t work. Try again from the main window.',

  settings: {
    sectionTitle: 'Meeting detection',
    sectionDescription: 'Offer to take notes when a call starts.',
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
    unsupported: 'Meeting detection is available on macOS.',
  },
} as const;
