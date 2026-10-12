/**
 * The mock settings document in memory: `settings.ts` reads and patches it,
 * other mocks that follow a setting (`calendar.ts`) read it. Kept apart
 * because a mock module exports exactly what its generated module does.
 */
import type * as Generated from '@/lib/ipc/generated/settings';

/**
 * Onboarding as `pnpm dev:mock` starts it: done, so the app opens, unless
 * the URL asks for the flow (`?onboarding`), which starts a fresh install's.
 */
function startingOnboarding(): Generated.OnboardingSettings {
  const fresh = new URLSearchParams(window.location.search).has('onboarding');
  return { completed: !fresh, step: 'welcome', outcomes: {} };
}

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
      showActionItemReminders: true,
      meetingReminderMinutes: [15, 5],
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
  consent: {
    chatMessage: {
      zoom: false,
      teams: false,
      meet: false,
      message:
        "Hi all, I'm transcribing this call with Deino for my own notes. It runs on my Mac and nothing is shared. Let me know if you'd rather I didn't.",
    },
    reminder: true,
  },
  privacy: {
    redactSensitive: false,
    encryptDatabase: false,
    appLock: { enabled: false, idleMinutes: 5, lockOnWake: true },
    retention: { audio: 'never', transcript: 'never', everything: 'never' },
    pauseTipSeen: false,
  },
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
  briefs: { enabled: true, audience: 'external' },
  calendar: {
    syncEnabled: true,
    hiddenCalendars: [],
    google: { clientId: '' },
    microsoft: { clientId: '' },
    autoRecord: { internal: false, external: true, organizedByOthers: true, notAccepted: false },
    reminders: { enabled: true, quietCalendars: [] },
  },
  integrations: { apps: {} },
  automations: { shellScripts: false },
  shortcuts: { record: 'Ctrl+Alt+Cmd+R', quickNote: 'Ctrl+Alt+Cmd+N', ask: 'Ctrl+Alt+Cmd+J' },
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
  onboarding: startingOnboarding(),
};

/** The settings as they are now. */
export function currentSettings(): Generated.Settings {
  return settings;
}

/** Replaces the settings after a patch. */
export function replaceSettings(next: Generated.Settings): void {
  settings = next;
}
