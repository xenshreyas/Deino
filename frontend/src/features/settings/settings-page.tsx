/**
 * The settings feature's own pages: the inherited panels FND-06 has not
 * moved yet, and the pages that combine several features' sections. Other
 * pages register from their feature's `settings-page.tsx`.
 */
import { About } from '@/components/About';
import { BetaSettings } from '@/components/BetaSettings';
import { PreferenceSettings } from '@/components/PreferenceSettings';
import { RecordingSettings } from '@/components/RecordingSettings';
import { SummaryModelSettings } from '@/components/SummaryModelSettings';
import { appearanceCopy, ThemePicker } from '@/features/appearance';
import { IntegrationsSettings } from '@/features/integrations';
import { NetworkSettings } from '@/features/network';
import {
  AudioLines,
  Cpu,
  FlaskConical,
  Info,
  Mic,
  Palette,
  Plug,
  Settings,
  ShieldCheck,
  Video,
} from '@/ui/icons';

import { CaptureModeSettings } from './components/capture-mode-settings';
import { EngineSettings } from './components/engine-settings';
import { DetectionSettingsPage } from './components/detection-settings';
import { RecordingBehaviour } from './components/recording-behaviour';
import { RedactionSettings } from './components/redaction-settings';
import { useOfflineMode } from './offline-mode';
import type { SettingsPage } from './sections';
import { useSaveTheme, useThemePreference } from './theme';

/** Appearance: the theme picker. */
function AppearanceSection() {
  const preference = useThemePreference();
  const saveTheme = useSaveTheme();
  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold text-fg">{appearanceCopy.themeLabel}</h2>
      <p className="text-xs text-fg-muted">{appearanceCopy.themeDescription}</p>
      <div className="pt-2">
        <ThemePicker value={preference} onChange={saveTheme} />
      </div>
    </section>
  );
}

/** Recording: the record panel and auto-stop, then the inherited audio settings. */
function RecordingSection() {
  return (
    <div className="flex flex-col gap-8">
      <RecordingBehaviour />
      <RecordingSettings />
    </div>
  );
}

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

/** Privacy → Network: secrets kept out of notes and exports (NOT-09), then
 * offline mode, services and the egress log (FND-12). */
function NetworkSection() {
  const { offlineMode, setOfflineMode } = useOfflineMode();
  return (
    <div className="flex flex-col gap-8">
      <RedactionSettings />
      <NetworkSettings offlineMode={offlineMode} onOfflineModeChange={setOfflineMode} />
    </div>
  );
}

/** Workspace → Integrations: connected apps, planned ones and what was sent (INT-00). */
function IntegrationsSection() {
  const { offlineMode } = useOfflineMode();
  return <IntegrationsSettings offlineMode={offlineMode} />;
}

/** Summary models: the inherited panel, which takes optional props. */
function SummaryModelsSection() {
  return <SummaryModelSettings />;
}

export const settingsPages = [
  {
    id: 'general',
    group: 'app',
    order: 10,
    icon: Settings,
    label: 'General',
    description: 'Notifications, storage and how Deino starts.',
    Component: PreferenceSettings,
  },
  {
    id: 'appearance',
    group: 'app',
    order: 20,
    icon: Palette,
    label: 'Appearance',
    description: 'Theme for every Deino window.',
    Component: AppearanceSection,
  },
  {
    id: 'recording',
    group: 'capture',
    order: 10,
    icon: Mic,
    label: 'Recording',
    description: 'Where recordings are saved and how audio is captured.',
    Component: RecordingSection,
  },
  {
    id: 'detection',
    group: 'capture',
    order: 20,
    icon: Video,
    label: 'Meeting detection',
    description: 'Offer to take notes when a call starts, and which apps to ignore.',
    Component: DetectionSettingsPage,
  },
  {
    id: 'transcription',
    group: 'capture',
    order: 30,
    icon: AudioLines,
    label: 'Transcription',
    description: 'Live or after the meeting, and the local speech-to-text engine and its models.',
    Component: TranscriptionSection,
  },
  {
    id: 'models',
    group: 'ai',
    order: 20,
    icon: Cpu,
    label: 'Summary models',
    description: 'The language model that writes your notes.',
    Component: SummaryModelsSection,
  },
  {
    id: 'integrations',
    group: 'workspace',
    order: 20,
    icon: Plug,
    label: 'Integrations',
    description: 'Send notes, tasks and messages to the tools you use.',
    Component: IntegrationsSection,
  },
  {
    id: 'network',
    group: 'privacy',
    order: 20,
    icon: ShieldCheck,
    label: 'Network',
    description: 'What may leave this Mac, what is hidden first, and a log of what left.',
    Component: NetworkSection,
  },
  {
    id: 'beta',
    group: 'more',
    order: 10,
    icon: FlaskConical,
    label: 'Beta',
    description: 'Features that are still being finished.',
    Component: BetaSettings,
  },
  {
    id: 'about',
    group: 'more',
    order: 20,
    icon: Info,
    label: 'About',
    description: 'Version, privacy and updates.',
    Component: About,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    appearance: true;
    recording: true;
    detection: true;
    transcription: true;
    models: true;
    integrations: true;
    network: true;
    beta: true;
    about: true;
  }
}
