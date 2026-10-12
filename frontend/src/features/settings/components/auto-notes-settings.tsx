/**
 * Settings → Advanced → Notes after meetings (UX-11): whether the pipeline
 * writes notes once a recording stops (`ai.autoSummary`). The everyday AI
 * choice is Notes → AI, which has no such switch; this one stays reachable
 * here so a library with it off is never stuck.
 */
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { autoNotesCopy as copy } from '../copy';
import { usePatchSettings, useSettings } from '../queries';
import { SwitchRow } from './switch-row';

/** The section. */
export function AutoNotesSettings() {
  const { data: settings } = useSettings();
  const patch = usePatchSettings();
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {settings === undefined ? (
        <SkeletonText lines={1} />
      ) : (
        <SettingsCard label={copy.title}>
          <SwitchRow
            id="auto-notes"
            label={copy.switchTitle}
            description={settings.ai.autoSummary ? copy.on : copy.off}
            checked={settings.ai.autoSummary}
            onChange={(autoSummary) => {
              patch.mutate({ ai: { autoSummary } });
            }}
          />
        </SettingsCard>
      )}
    </SettingsSection>
  );
}
