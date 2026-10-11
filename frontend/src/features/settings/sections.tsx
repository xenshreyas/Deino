/**
 * The settings pages, in nav order (ui.md §3 Settings layout). A card that
 * adds settings adds a page here, or a section inside an existing page.
 * Most pages still render inherited settings panels; FND-06 moves their
 * values into the typed settings store.
 */
import { type ComponentType, lazy, Suspense } from 'react';

import { About } from '@/components/About';
import { BetaSettings } from '@/components/BetaSettings';
import { PreferenceSettings } from '@/components/PreferenceSettings';
import { RecordingSettings } from '@/components/RecordingSettings';
import { SummaryModelSettings } from '@/components/SummaryModelSettings';
import { TranscriptSettings } from '@/components/TranscriptSettings';
import { useConfig } from '@/contexts/ConfigContext';
import { appearanceCopy, ThemePicker } from '@/features/appearance';
import { CalendarSettings } from '@/features/calendar';
import { ExportSettingsPage } from '@/features/export';
import { IntegrationsSettings } from '@/features/integrations';
import { AiSettings } from '@/features/llm';
import { NetworkSettings } from '@/features/network';
import { ProfilePage } from '@/features/profile';
import { IndexSettings } from '@/features/search';
import { PeopleSettings } from '@/features/speakers';
import { TemplatesPage } from '@/features/templates';
import {
  AudioLines,
  Calendar,
  Cpu,
  Download,
  FlaskConical,
  type Icon,
  Info,
  LayoutTemplate,
  Mic,
  Palette,
  Plug,
  Search,
  Settings,
  ShieldCheck,
  Sparkles,
  UserRound,
  Users,
  Video,
  Wrench,
} from '@/ui/icons';

import { DetectionSettingsPage } from './components/detection-settings';
import { settingsCopy } from './copy';
import { useOfflineMode } from './offline-mode';
import type { SettingsSectionId } from './search';
import { useSaveTheme, useThemePreference } from './theme';

/** A nav group. */
export type SettingsGroup = keyof typeof settingsCopy.groups;

/** One settings page. */
export interface SettingsSection {
  id: SettingsSectionId;
  group: SettingsGroup;
  icon: Icon;
  Component: ComponentType;
}

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

/** Transcription: the inherited engine picker over the config context. */
function TranscriptionSection() {
  const { transcriptModelConfig, setTranscriptModelConfig } = useConfig();
  return (
    <TranscriptSettings
      transcriptModelConfig={transcriptModelConfig}
      setTranscriptModelConfig={setTranscriptModelConfig}
    />
  );
}

/** Privacy → Network: offline mode, services and the egress log (FND-12). */
function NetworkSection() {
  const { offlineMode, setOfflineMode } = useOfflineMode();
  return <NetworkSettings offlineMode={offlineMode} onOfflineModeChange={setOfflineMode} />;
}

/** Workspace → Integrations: connected apps, planned ones and what was sent (INT-00). */
function IntegrationsSection() {
  const { offlineMode } = useOfflineMode();
  return <IntegrationsSettings offlineMode={offlineMode} />;
}

/** Settings → Developer (FND-04), loaded lazily so release builds drop it. */
const DeveloperSettings = import.meta.env.DEV
  ? lazy(() =>
      import('@/features/developer').then((module) => ({ default: module.DeveloperSettings })),
    )
  : null;

/** Developer: fixture seeding, debug builds only. */
function DeveloperSection() {
  if (DeveloperSettings === null) return null;
  return (
    <Suspense fallback={null}>
      <DeveloperSettings />
    </Suspense>
  );
}

/** Pages that exist only in debug builds (the `dev` plugin is absent from release). */
const DEBUG_SECTIONS: readonly SettingsSection[] = import.meta.env.DEV
  ? [{ id: 'developer', group: 'more', icon: Wrench, Component: DeveloperSection }]
  : [];

/** Every page, in order. */
export const SETTINGS_SECTIONS: readonly SettingsSection[] = [
  { id: 'general', group: 'app', icon: Settings, Component: PreferenceSettings },
  { id: 'appearance', group: 'app', icon: Palette, Component: AppearanceSection },
  { id: 'recording', group: 'capture', icon: Mic, Component: () => <RecordingSettings /> },
  { id: 'detection', group: 'capture', icon: Video, Component: DetectionSettingsPage },
  { id: 'transcription', group: 'capture', icon: AudioLines, Component: TranscriptionSection },
  { id: 'people', group: 'capture', icon: Users, Component: PeopleSettings },
  { id: 'ai', group: 'ai', icon: Sparkles, Component: AiSettings },
  { id: 'models', group: 'ai', icon: Cpu, Component: () => <SummaryModelSettings /> },
  { id: 'templates', group: 'ai', icon: LayoutTemplate, Component: TemplatesPage },
  { id: 'profile', group: 'ai', icon: UserRound, Component: ProfilePage },
  { id: 'search', group: 'ai', icon: Search, Component: IndexSettings },
  { id: 'calendar', group: 'workspace', icon: Calendar, Component: CalendarSettings },
  { id: 'integrations', group: 'workspace', icon: Plug, Component: IntegrationsSection },
  { id: 'network', group: 'privacy', icon: ShieldCheck, Component: NetworkSection },
  { id: 'export', group: 'data', icon: Download, Component: ExportSettingsPage },
  { id: 'beta', group: 'more', icon: FlaskConical, Component: BetaSettings },
  { id: 'about', group: 'more', icon: Info, Component: About },
  ...DEBUG_SECTIONS,
];
