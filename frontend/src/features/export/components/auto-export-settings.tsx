import { type FocusEvent, useId } from 'react';
import { toast } from 'sonner';

import { usePatchSettings } from '@/features/settings';
import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { FolderOpen, ICON_PROPS } from '@/ui/icons';
import { Input } from '@/ui/input';
import { MoreOptions } from '@/ui/more-options';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';

import { exportCopy } from '../copy';
import { messageOf } from '../hooks/use-share-actions';
import { useChooseExportFolder } from '../queries';
import type { ExportSettings } from '../types';
import { SectionToggles } from './section-toggles';

/** The switches of the auto-export card, each a boolean of `ExportSettings`. */
type SwitchKey = 'autoExport' | 'obsidian' | 'transcriptSidecar' | 'overwrite';

/** A change to the export section; the folder only changes through its dialog. */
type ExportChange = Partial<Omit<ExportSettings, 'folder'>>;

/** Saves part of the export section, explaining a rejected value. */
function useSaveExportSettings() {
  const patch = usePatchSettings();
  return (change: ExportChange) => {
    patch.mutate({ export: change }, { onError: (error) => toast.error(messageOf(error)) });
  };
}

/** What every row of the card receives. */
interface RowProps<K extends keyof ExportSettings> {
  settings: ExportSettings;
  field: K;
  label: string;
  hint: string;
}

/** A switch row bound to one boolean of the export settings. */
function SwitchRow({ settings, field, label, hint }: RowProps<SwitchKey>) {
  const id = useId();
  const save = useSaveExportSettings();
  return (
    <SettingsRow label={label} description={hint} htmlFor={id}>
      <Switch
        id={id}
        checked={settings[field]}
        onCheckedChange={(checked) => {
          save({ [field]: checked });
        }}
      />
    </SettingsRow>
  );
}

/** A template field saved when it loses focus, if it changed. */
function TemplateRow({ settings, field, label, hint }: RowProps<'fileName' | 'subfolder'>) {
  const id = useId();
  const save = useSaveExportSettings();
  const commit = (event: FocusEvent<HTMLInputElement>) => {
    const value = event.target.value.trim();
    if (value !== settings[field]) save({ [field]: value });
  };
  return (
    <SettingsRow label={label} description={hint} htmlFor={id}>
      <Input
        id={id}
        key={settings[field]}
        defaultValue={settings[field]}
        onBlur={commit}
        spellCheck={false}
        className="w-[220px] font-mono text-xs"
      />
    </SettingsRow>
  );
}

/** The export folder, chosen only through the folder dialog. */
function FolderRow({ folder }: { folder: string | null }) {
  const choose = useChooseExportFolder();
  const pick = () => {
    choose.mutate(undefined, { onError: (error) => toast.error(messageOf(error)) });
  };
  return (
    <SettingsRow label={exportCopy.folderLabel} description={folder ?? exportCopy.folderNone}>
      <Button variant="secondary" size="sm" disabled={choose.isPending} onClick={pick}>
        <FolderOpen {...ICON_PROPS} />
        {folder === null ? exportCopy.folderChoose : exportCopy.folderChange}
      </Button>
    </SettingsRow>
  );
}

/** Where notes go: the name templates. */
function WhereRows({ settings }: { settings: ExportSettings }) {
  return (
    <>
      <TemplateRow
        settings={settings}
        field="fileName"
        label={exportCopy.fileNameLabel}
        hint={exportCopy.fileNameHint}
      />
      <TemplateRow
        settings={settings}
        field="subfolder"
        label={exportCopy.subfolderLabel}
        hint={exportCopy.subfolderHint}
      />
    </>
  );
}

/** What notes hold: the sections, Obsidian mode, the sidecar, overwriting. */
function WhatRows({ settings }: { settings: ExportSettings }) {
  const save = useSaveExportSettings();
  return (
    <>
      <div className="flex flex-col gap-2 px-4 py-3">
        <span className="text-sm text-fg">{exportCopy.sectionsLabel}</span>
        <SectionToggles
          value={settings.sections}
          label={exportCopy.sectionsLabel}
          onChange={(sections) => {
            save({ sections });
          }}
        />
      </div>
      <SwitchRow
        settings={settings}
        field="obsidian"
        label={exportCopy.obsidianLabel}
        hint={exportCopy.obsidianHint}
      />
      <SwitchRow
        settings={settings}
        field="transcriptSidecar"
        label={exportCopy.sidecarLabel}
        hint={exportCopy.sidecarHint}
      />
      <SwitchRow
        settings={settings}
        field="overwrite"
        label={exportCopy.overwriteLabel}
        hint={exportCopy.overwriteHint}
      />
    </>
  );
}

/**
 * Settings → Connections → Auto-export (EXP-02): the switch and the folder,
 * then names and contents under "More options" (UX-15).
 */
export function AutoExportSettings({ settings }: { settings: ExportSettings }) {
  return (
    <SettingsSection title={exportCopy.autoTitle} description={exportCopy.autoDescription}>
      <SettingsCard label={exportCopy.autoTitle}>
        <SwitchRow
          settings={settings}
          field="autoExport"
          label={exportCopy.autoToggle}
          hint={exportCopy.autoToggleHint}
        />
        <FolderRow folder={settings.folder} />
      </SettingsCard>
      <MoreOptions>
        <SettingsCard label={exportCopy.moreTitle}>
          <WhereRows settings={settings} />
          <WhatRows settings={settings} />
        </SettingsCard>
      </MoreOptions>
    </SettingsSection>
  );
}
