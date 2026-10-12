/**
 * The settings feature's registrations (REG-02, UX-19): the nine host pages
 * every feature places its sections on, and the settings feature's own
 * sections, including the inherited panels FND-06 has not moved yet.
 * Other sections register from their feature's `settings-page.tsx`.
 */
import { About } from '@/components/About';
import { RecordingsFolderSettings } from '@/components/RecordingsFolderSettings';
import { IntegrationsSettings } from '@/features/integrations';
import { NetworkServices, NetworkSettings } from '@/features/network';
import {
  Calendar,
  Info,
  LayoutTemplate,
  Mic,
  NotebookPen,
  Plug,
  Settings,
  ShieldCheck,
  SlidersHorizontal,
} from '@/ui/icons';

import { SettingsSection } from '@/ui/settings-section';

import { AppearanceSection } from './components/appearance-section';
import { AutoNotesSettings } from './components/auto-notes-settings';
import { CaptureModeSettings } from './components/capture-mode-settings';
import { EngineSettings } from './components/engine-settings';
import { DetectionSettingsPage } from './components/detection-settings';
import { NotificationsSettings } from './components/notifications-settings';
import { AutoStopSettings, RecordingBehaviour } from './components/recording-behaviour';
import { RedactionSettings } from './components/redaction-settings';
import { transcriptionSectionCopy } from './copy';
import { useOfflineMode } from './offline-mode';
import type { SettingsPage, SettingsPageSection } from './sections';

/** Transcription: live or after the meeting (CAP-08), then the engine and
 * its model, read from and saved to the settings the recorder uses. */
function TranscriptionSection() {
  return (
    <div className="flex flex-col gap-8">
      <CaptureModeSettings />
      <EngineSettings />
    </div>
  );
}

/**
 * Advanced → Transcription: the transcription panels as one unit under a
 * heading. A card added to Transcription (another engine) goes inside
 * {@link TranscriptionSection} or registers as its own section at order 25.
 */
function TranscriptionPage() {
  return (
    <SettingsSection
      title={transcriptionSectionCopy.title}
      description={transcriptionSectionCopy.description}
    >
      <TranscriptionSection />
    </SettingsSection>
  );
}

/** Privacy → Network: offline mode and the log of what left this Mac (FND-12). */
function NetworkSection() {
  const { offlineMode, setOfflineMode } = useOfflineMode();
  return <NetworkSettings offlineMode={offlineMode} onOfflineModeChange={setOfflineMode} />;
}

/** Connections → Connected apps: what can be sent where, and what was (INT-00). */
function IntegrationsSection() {
  const { offlineMode } = useOfflineMode();
  return <IntegrationsSettings offlineMode={offlineMode} />;
}

/** The nine host pages, in nav order (UX-19). */
export const settingsPages = [
  {
    id: 'general',
    order: 10,
    icon: Settings,
    label: 'General',
    description: 'Theme, notifications and where recordings are kept.',
  },
  {
    id: 'recording',
    order: 20,
    icon: Mic,
    label: 'Recording',
    description: 'Permissions, the microphone, and what happens around each call.',
  },
  {
    id: 'notes',
    order: 30,
    icon: NotebookPen,
    label: 'Notes',
    description: 'Who writes your notes, what they know about you, and how names are spelled.',
  },
  {
    id: 'templates',
    order: 40,
    icon: LayoutTemplate,
    label: 'Templates',
    description: 'What your notes contain for each kind of meeting, and when each is used.',
  },
  {
    id: 'calendar',
    order: 50,
    icon: Calendar,
    label: 'Calendar',
    description: 'Your calendars, which meetings record on their own, and briefs before them.',
  },
  {
    id: 'connections',
    order: 60,
    icon: Plug,
    label: 'Connections',
    description: 'Apps your notes go to, exports, and meetings brought in from elsewhere.',
  },
  {
    id: 'privacy',
    order: 70,
    icon: ShieldCheck,
    label: 'Privacy',
    description: 'Lock and encryption, what is deleted when, and what leaves this Mac.',
  },
  {
    id: 'advanced',
    order: 80,
    icon: SlidersHorizontal,
    label: 'Advanced',
    description: 'For the few who need them: models, transcription, services and developer tools.',
  },
  {
    id: 'about',
    order: 90,
    icon: Info,
    label: 'About',
    description: 'Version, privacy and licences.',
  },
] satisfies readonly SettingsPage[];

/** The settings feature's own sections. */
export const settingsSections = [
  { id: 'appearance', page: 'general', order: 10, Component: AppearanceSection },
  { id: 'notifications', page: 'general', order: 20, Component: NotificationsSettings },
  { id: 'recordings-folder', page: 'general', order: 30, Component: RecordingsFolderSettings },
  { id: 'recording', page: 'recording', order: 20, Component: RecordingBehaviour },
  { id: 'detection', page: 'recording', order: 30, Component: DetectionSettingsPage },
  { id: 'auto-stop', page: 'recording', order: 40, Component: AutoStopSettings },
  { id: 'integrations', page: 'connections', order: 10, Component: IntegrationsSection },
  { id: 'redaction', page: 'privacy', order: 20, Component: RedactionSettings },
  { id: 'network', page: 'privacy', order: 30, Component: NetworkSection },
  { id: 'auto-notes', page: 'advanced', order: 5, Component: AutoNotesSettings },
  { id: 'transcription', page: 'advanced', order: 20, Component: TranscriptionPage },
  { id: 'network-services', page: 'advanced', order: 60, Component: NetworkServices },
  { id: 'about', page: 'about', order: 10, Component: About },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    recording: true;
    notes: true;
    templates: true;
    calendar: true;
    connections: true;
    privacy: true;
    advanced: true;
    about: true;
    appearance: true;
    notifications: true;
    'recordings-folder': true;
    detection: true;
    'auto-stop': true;
    integrations: true;
    redaction: true;
    network: true;
    'auto-notes': true;
    transcription: true;
    'network-services': true;
  }
}
