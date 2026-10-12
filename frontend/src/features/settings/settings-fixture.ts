/**
 * The default settings as Rust returns them, for tests. Keep it equal to
 * `Settings::default()` (`crates/deino-core/src/settings/model.rs`); the
 * generated `Settings` type keeps its shape honest.
 */
import type { Settings } from './types';

const DEFAULT_SETTINGS: Settings = {
  version: 1,
  ai: {
    provider: 'builtin-ai',
    model: '',
    providerModels: {},
    ollamaEndpoint: null,
    customOpenai: null,
    autoSummary: false,
    routes: {},
  },
  audio: { preferredMicDevice: null, preferredSystemDevice: null, usesHeadphones: false },
  recording: {
    autoSave: true,
    showRecordingNotice: true,
    showPanel: true,
    panelPosition: null,
    autoStop: { enabled: true, afterCalendarEnd: false, offerNextMeeting: true },
  },
  transcription: {
    engine: 'parakeet_unified',
    model: '',
    liveTranscription: true,
    highAccuracyRepass: false,
  },
  notifications: {
    recordingNotifications: true,
    timeBasedReminders: true,
    meetingReminders: true,
    respectDoNotDisturb: true,
    notificationSound: true,
    systemPermissionGranted: false,
    consentGiven: false,
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
      showActionItemReminders: true,
    },
  },
  detection: {
    enabled: true,
    ignoredApps: [],
    autoStart: { enabled: false, countdownSeconds: 10 },
  },
  speakers: {
    autoIdentify: true,
    thresholds: { autoAssign: 0.72, autoAssignNonAttendee: 0.77, margin: 0.08, suggest: 0.55 },
  },
  privacy: {
    redactSensitive: false,
    encryptDatabase: false,
    appLock: { enabled: false, idleMinutes: 5, lockOnWake: true },
  },
  network: { offlineMode: false, services: {} },
  profile: {
    name: '',
    emails: [],
    role: '',
    company: '',
    focusAreas: [],
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
  calendar: {
    syncEnabled: false,
    hiddenCalendars: [],
    google: { clientId: '' },
    microsoft: { clientId: '' },
  },
  integrations: { apps: {} },
  automations: { shellScripts: false },
  ui: {
    theme: 'system',
    showConfidenceIndicator: true,
    transcriptPaneRatio: 0.3,
    meetingsList: {
      view: null,
      filter: {
        dateRange: null,
        people: [],
        companies: [],
        tags: [],
        folders: [],
        hasActionItems: false,
        audience: null,
      },
    },
  },
  beta: { importAndRetranscribe: true },
  onboarding: { completed: false, step: 'welcome', outcomes: {} },
};

/** A fresh copy of the default settings. */
export function defaultSettings(): Settings {
  return structuredClone(DEFAULT_SETTINGS);
}
