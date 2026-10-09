/**
 * Mock of `@/lib/ipc/generated/detection`: one Google Meet call in Chrome,
 * linked to a calendar event, so `pnpm dev:mock` can show the prompt at
 * `/detection-prompt.html` (add `?autostart` for the countdown, `?kind=huddle`
 * or `?kind=call` for the other titles). Answers close it; "Always ignore"
 * adds Chrome to the mock settings' ignore list.
 *
 * It imports no feature code: the detection feature reaches this module
 * through its generated bindings, so importing it back would form a cycle.
 */
import type * as Generated from '@/lib/ipc/generated/detection';
import { unwrap } from '@/lib/ipc/result';

import { failure, ok } from './command-result';
import { latency } from './latency';
import { mockEvent } from './mock-event';
import { commands as settings } from './settings';

/** How long the mock auto-start counts down, in milliseconds. */
const COUNTDOWN_MS = 10_000;

/** Chrome, as Rust names it. */
const CHROME = { bundleId: 'com.google.Chrome' as Generated.BundleId, name: 'Google Chrome' };

/** The linked event's identifier. */
const EVENT_ID = '01926b2c-0000-7000-8000-000000000001' as Generated.CalendarEventId;

/** The kind the page asked for (`?kind=`), a meeting by default. */
function requestedKind(params: URLSearchParams): Generated.MeetingKind {
  const kind = params.get('kind');
  return kind === 'huddle' || kind === 'call' ? kind : 'meeting';
}

/** The detection on screen, built from the page's query string. */
function fixtureDetection(): Generated.Detection {
  const params = new URLSearchParams(window.location.search);
  const now = Date.now();
  return {
    id: 1,
    app: CHROME,
    kind: requestedKind(params),
    platform: 'googleMeet',
    event: {
      id: EVENT_ID,
      title: 'Design review: onboarding v2',
      startsAt: new Date(now - 2 * 60_000).toISOString(),
    },
    detectedAt: new Date(now).toISOString(),
    autoStartAt: params.has('autostart') ? new Date(now + COUNTDOWN_MS).toISOString() : null,
  };
}

let shown: Generated.Detection | null = null;

export const events = {
  meetingDetected: mockEvent<Generated.MeetingDetected>('plugin:detection:meeting-detected'),
  meetingEnded: mockEvent<Generated.MeetingEnded>('plugin:detection:meeting-ended'),
  promptClosed: mockEvent<Generated.PromptClosed>('plugin:detection:prompt-closed'),
  recordingRequested: mockEvent<Generated.RecordingRequested>(
    'plugin:detection:recording-requested',
  ),
} satisfies typeof Generated.events;

/** Closes prompt `id`; `null` when it already closed. */
function close(id: Generated.DetectionId): Generated.Detection | null {
  if (shown?.id !== id) return null;
  const closed = shown;
  shown = null;
  void events.promptClosed.emit({ id });
  return closed;
}

/** The error Rust returns for an answer to a closed prompt. */
const alreadyClosed = () =>
  failure<null>({ code: 'not_found', message: 'that meeting prompt has already closed' });

export const commands = {
  async current() {
    await latency();
    shown ??= fixtureDetection();
    return ok(shown);
  },
  async takeNotes(id) {
    await latency();
    const detection = close(id);
    if (detection === null) return alreadyClosed();
    void events.recordingRequested.emit({ detection, openNotes: true });
    return ok(null);
  },
  async dismiss(id) {
    await latency();
    return close(id) === null ? alreadyClosed() : ok(null);
  },
  async ignoreApp(id) {
    await latency();
    const detection = close(id);
    if (detection === null) return alreadyClosed();
    const { detection: saved } = await unwrap(settings.get());
    await settings.patch({ detection: { ignoredApps: [...saved.ignoredApps, detection.app] } });
    return ok(null);
  },
  async settings() {
    await latency();
    const { detection } = await unwrap(settings.get());
    return ok({ settings: detection, supported: true, readsBrowserUrls: false });
  },
} satisfies typeof Generated.commands;
