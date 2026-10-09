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
import { AiSettings } from '@/features/llm';
import {
  AudioLines,
  Cpu,
  FlaskConical,
  type Icon,
  Info,
  Mic,
  Palette,
  Settings,
  Sparkles,
  Wrench,
} from '@/ui/icons';

import { settingsCopy } from './copy';
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
  { id: 'transcription', group: 'capture', icon: AudioLines, Component: TranscriptionSection },
  { id: 'ai', group: 'ai', icon: Sparkles, Component: AiSettings },
  { id: 'models', group: 'ai', icon: Cpu, Component: () => <SummaryModelSettings /> },
  { id: 'beta', group: 'more', icon: FlaskConical, Component: BetaSettings },
  { id: 'about', group: 'more', icon: Info, Component: About },
  ...DEBUG_SECTIONS,
];
