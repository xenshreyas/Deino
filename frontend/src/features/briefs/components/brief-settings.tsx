/**
 * Settings → Calendar → Pre-meeting briefs (NOT-08): whether briefs are
 * prepared, and for which meetings (`briefs.enabled`, `briefs.audience`).
 */
import { toast } from 'sonner';

import { type Settings, usePatchSettings, useSettings } from '@/features/settings';
import { SegmentedControl } from '@/ui/segmented-control';
import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { briefsCopy } from '../copy';

const copy = briefsCopy.settings;

type BriefSettings = Settings['briefs'];

const AUDIENCES = [
  { value: 'external', label: copy.audience.external },
  { value: 'groups', label: copy.audience.groups },
] as const;

/** The two controls over the saved settings. */
function BriefCard({ briefs }: { briefs: BriefSettings }) {
  const patch = usePatchSettings();
  const save = (next: Partial<BriefSettings>) => {
    patch.mutate(
      { briefs: next },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
  return (
    <SettingsCard label={copy.title}>
      <SettingsRow
        label={copy.enabledTitle}
        description={briefs.enabled ? copy.enabledOn : copy.enabledOff}
        htmlFor="briefs-enabled"
      >
        <Switch
          id="briefs-enabled"
          checked={briefs.enabled}
          disabled={patch.isPending}
          onCheckedChange={(enabled) => {
            save({ enabled });
          }}
        />
      </SettingsRow>
      <SettingsRow label={copy.audienceTitle}>
        <SegmentedControl
          label={copy.audienceTitle}
          segments={AUDIENCES}
          value={briefs.audience}
          onValueChange={(audience) => {
            save({ audience });
          }}
        />
      </SettingsRow>
    </SettingsCard>
  );
}

/** "Pre-meeting briefs", once the settings are loaded. */
export function BriefSettingsSection() {
  const settings = useSettings();
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {settings.data === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <BriefCard briefs={settings.data.briefs} />
      )}
    </SettingsSection>
  );
}
