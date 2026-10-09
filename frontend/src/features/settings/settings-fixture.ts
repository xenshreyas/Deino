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
  audio: { preferredMicDevice: null, preferredSystemDevice: null },
  recording: { autoSave: true, showRecordingNotice: true },
  transcription: { engine: 'parakeet_unified', model: '' },
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
    },
  },
  privacy: {},
  profile: {},
  ui: { theme: 'system', showConfidenceIndicator: true, transcriptPaneRatio: 0.3 },
  beta: { importAndRetranscribe: true },
  onboarding: {
    completed: false,
    currentStep: 1,
    parakeet: 'not_downloaded',
    summary: 'not_downloaded',
    selectedSummaryModel: null,
    updatedAt: null,
  },
};

/** A fresh copy of the default settings. */
export function defaultSettings(): Settings {
  return structuredClone(DEFAULT_SETTINGS);
}
