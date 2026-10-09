/**
 * Mock of `@/lib/ipc/generated/settings`: the settings in memory, changed by
 * RFC 7396 merge patches, with `SettingsChanged` after every change.
 *
 * It starts past onboarding, so `pnpm dev:mock` opens the app. It does not
 * repeat the Rust validation: a patch the UI could not have typed is the
 * Rust contract tests' job.
 */
import type * as Generated from '@/lib/ipc/generated/settings';

import { latency } from './latency';
import { ok } from './command-result';
import { mockEvent } from './mock-event';

/** The settings a returning user has: onboarding done, Claude for notes. */
let settings: Generated.Settings = {
  version: 1,
  ai: {
    provider: 'claude',
    model: 'claude-sonnet-4-5',
    providerModels: { claude: 'claude-sonnet-4-5' },
    ollamaEndpoint: null,
    customOpenai: null,
    autoSummary: false,
    routes: {},
  },
  audio: { preferredMicDevice: null, preferredSystemDevice: null, usesHeadphones: false },
  recording: { autoSave: true, showRecordingNotice: true },
  transcription: { engine: 'parakeet_unified', model: '' },
  notifications: {
    recordingNotifications: true,
    timeBasedReminders: true,
    meetingReminders: true,
    respectDoNotDisturb: true,
    notificationSound: true,
    systemPermissionGranted: true,
    consentGiven: true,
    manualDndMode: false,
    preferences: {
      showRecordingStarted: false,
      showRecordingStopped: false,
      showRecordingPaused: true,
      showRecordingResumed: true,
      showTranscriptionComplete: true,
      showMeetingReminders: true,
      showSystemErrors: true,
      meetingReminderMinutes: [15, 5],
    },
  },
  detection: {
    enabled: true,
    ignoredApps: [],
    autoStart: { enabled: false, countdownSeconds: 10 },
  },
  privacy: {},
  network: { offlineMode: false, services: {} },
  profile: {
    name: 'Shreyas',
    emails: ['shreyas@deino.dev' as Generated.Email],
    role: 'CTO',
    company: 'Deino',
    focusAreas: ['Reliability', 'Hiring'],
    writingStyle: '',
    context: '',
  },
  templates: { defaultTemplate: null },
  organize: { autoTag: true },
  export: {
    autoExport: false,
    folder: null,
    fileName: '{date} {title}.md',
    subfolder: '{yyyy}/{mm}',
    sections: {
      notes: true,
      myNotes: true,
      actionItems: true,
      insights: true,
      transcript: false,
      attendees: true,
    },
    overwrite: true,
    transcriptSidecar: false,
    obsidian: false,
  },
  calendar: { syncEnabled: true, hiddenCalendars: [] },
  ui: { theme: 'system', showConfidenceIndicator: true, transcriptPaneRatio: 0.3 },
  beta: { importAndRetranscribe: true },
  onboarding: {
    completed: true,
    currentStep: 4,
    parakeet: 'downloaded',
    summary: 'downloaded',
    selectedSummaryModel: null,
    updatedAt: '2026-10-01T09:00:00Z',
  },
};

export const events = {
  settingsChanged: mockEvent<Generated.SettingsChanged>('plugin:settings:settings-changed'),
} satisfies typeof Generated.events;

export const commands = {
  async get() {
    await latency();
    return ok(structuredClone(settings));
  },
  async patch(patch) {
    await latency();
    // The UI only sends typed `SettingsPatch` values, so the merge keeps the shape.
    settings = mergePatch(settings, patch) as Generated.Settings;
    void events.settingsChanged.emit({ settings: structuredClone(settings) });
    return ok(structuredClone(settings));
  },
} satisfies typeof Generated.commands;

/** RFC 7396: objects merge key by key, anything else replaces the target. */
function mergePatch(target: unknown, patch: unknown): unknown {
  if (!isRecord(patch)) return patch;
  const merged: Record<string, unknown> = isRecord(target) ? { ...target } : {};
  for (const [key, value] of Object.entries(patch)) {
    merged[key] = value === null ? null : mergePatch(merged[key], value);
  }
  return merged;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
