/**
 * The settings pages, in nav order (ui.md §3 Settings layout). A card that
 * adds settings adds a page here, or a section inside an existing page.
 * Most pages still render inherited settings panels; FND-06 moves their
 * values into the typed settings store.
 */
import type { ComponentType } from 'react';

import { About } from '@/components/About';
import { BetaSettings } from '@/components/BetaSettings';
import { PreferenceSettings } from '@/components/PreferenceSettings';
import { RecordingSettings } from '@/components/RecordingSettings';
import { SummaryModelSettings } from '@/components/SummaryModelSettings';
import { TranscriptSettings } from '@/components/TranscriptSettings';
import { useConfig } from '@/contexts/ConfigContext';
import { appearanceCopy, ThemePicker } from '@/features/appearance';
import { BriefSettingsSection } from '@/features/briefs';
import { CalendarSettings } from '@/features/calendar';
import { ConsentSettings } from '@/features/consent';
import { DeveloperSettings } from '@/features/developer';
import { ExportSettingsPage } from '@/features/export';
import { HotkeySettings } from '@/features/hotkeys';
import { IntegrationsSettings } from '@/features/integrations';
import { AiSettings } from '@/features/llm';
import { NetworkSettings } from '@/features/network';
import { ResumeSetup } from '@/features/onboarding';
import { PrivacySettings } from '@/features/privacy';
import { ProfilePage } from '@/features/profile';
import { WatchedFoldersSettings } from '@/features/import';
import { ImportSettings } from '@/features/importers';
import { IndexSettings } from '@/features/search';
import { ScreenCaptureSettings } from '@/features/screen';
import { SpeakerTagsSettings } from '@/features/speaker-tags';
import { PeopleSettings } from '@/features/speakers';
import { TemplatesPage } from '@/features/templates';
import { VocabularySettings } from '@/features/vocabulary';
import {
  AudioLines,
  Calendar,
  Contact,
  Cpu,
  Download,
  FlaskConical,
  FolderInput,
  type Icon,
  Inbox,
  Info,
  Keyboard,
  LayoutTemplate,
  Lock,
  Megaphone,
  Mic,
  Monitor,
  Palette,
  Plug,
  Rocket,
  Search,
  Settings,
  ShieldCheck,
  Sparkles,
  SpellCheck,
  UserRound,
  Users,
  Video,
  Wrench,
} from '@/ui/icons';

import { CaptureModeSettings } from './components/capture-mode-settings';
import { DetectionSettingsPage } from './components/detection-settings';
import { RecordingBehaviour } from './components/recording-behaviour';
import { RedactionSettings } from './components/redaction-settings';
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

/** Recording: the record panel and auto-stop, then the inherited audio settings. */
function RecordingSection() {
  return (
    <div className="flex flex-col gap-8">
      <RecordingBehaviour />
      <RecordingSettings />
    </div>
  );
}

/** Transcription: live or after the meeting (CAP-08), then the inherited
 * engine picker over the config context. */
function TranscriptionSection() {
  const { transcriptModelConfig, setTranscriptModelConfig } = useConfig();
  return (
    <div className="flex flex-col gap-8">
      <CaptureModeSettings />
      <TranscriptSettings
        transcriptModelConfig={transcriptModelConfig}
        setTranscriptModelConfig={setTranscriptModelConfig}
      />
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

/** Workspace → Calendar: sync and calendars (ORG-01), then pre-meeting briefs (NOT-08). */
function CalendarSection() {
  return (
    <div className="flex flex-col gap-8">
      <CalendarSettings />
      <BriefSettingsSection />
    </div>
  );
}

/** Workspace → Integrations: connected apps, planned ones and what was sent (INT-00). */
function IntegrationsSection() {
  const { offlineMode } = useOfflineMode();
  return <IntegrationsSettings offlineMode={offlineMode} />;
}

/** Every page, in order. */
export const SETTINGS_SECTIONS: readonly SettingsSection[] = [
  { id: 'general', group: 'app', icon: Settings, Component: PreferenceSettings },
  { id: 'appearance', group: 'app', icon: Palette, Component: AppearanceSection },
  { id: 'shortcuts', group: 'app', icon: Keyboard, Component: HotkeySettings },
  { id: 'setup', group: 'app', icon: Rocket, Component: ResumeSetup },
  { id: 'recording', group: 'capture', icon: Mic, Component: RecordingSection },
  { id: 'detection', group: 'capture', icon: Video, Component: DetectionSettingsPage },
  { id: 'transcription', group: 'capture', icon: AudioLines, Component: TranscriptionSection },
  { id: 'vocabulary', group: 'capture', icon: SpellCheck, Component: VocabularySettings },
  {
    id: 'watched-folders',
    group: 'capture',
    icon: FolderInput,
    Component: WatchedFoldersSettings,
  },
  { id: 'people', group: 'capture', icon: Users, Component: PeopleSettings },
  { id: 'speaker-tags', group: 'capture', icon: Contact, Component: SpeakerTagsSettings },
  { id: 'consent', group: 'capture', icon: Megaphone, Component: ConsentSettings },
  { id: 'screen', group: 'capture', icon: Monitor, Component: ScreenCaptureSettings },
  { id: 'ai', group: 'ai', icon: Sparkles, Component: AiSettings },
  { id: 'models', group: 'ai', icon: Cpu, Component: () => <SummaryModelSettings /> },
  { id: 'templates', group: 'ai', icon: LayoutTemplate, Component: TemplatesPage },
  { id: 'profile', group: 'ai', icon: UserRound, Component: ProfilePage },
  { id: 'search', group: 'ai', icon: Search, Component: IndexSettings },
  { id: 'calendar', group: 'workspace', icon: Calendar, Component: CalendarSection },
  { id: 'integrations', group: 'workspace', icon: Plug, Component: IntegrationsSection },
  { id: 'security', group: 'privacy', icon: Lock, Component: PrivacySettings },
  { id: 'network', group: 'privacy', icon: ShieldCheck, Component: NetworkSection },
  { id: 'import', group: 'data', icon: Inbox, Component: ImportSettings },
  { id: 'export', group: 'data', icon: Download, Component: ExportSettingsPage },
  { id: 'beta', group: 'more', icon: FlaskConical, Component: BetaSettings },
  { id: 'about', group: 'more', icon: Info, Component: About },
  { id: 'developer', group: 'more', icon: Wrench, Component: DeveloperSettings },
];
