/**
 * Settings → Network → Sensitive information (NOT-09): one switch over
 * `privacy.redactSensitive`. When on, Rust replaces passwords, API keys,
 * card numbers, SSNs and one-time codes with `[redacted]` in the notes'
 * input, exports and the `deino` CLI; the transcript keeps them.
 */
import { toast } from 'sonner';

import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { settingsCopy } from '../copy';
import { usePatchSettings, useSettings } from '../queries';

const copy = settingsCopy.redaction;

/** The switch over the saved setting. */
function RedactionCard({ enabled }: { enabled: boolean }) {
  const patch = usePatchSettings();
  return (
    <SettingsCard label={copy.title}>
      <SettingsRow
        label={copy.switchTitle}
        description={enabled ? copy.on : copy.off}
        htmlFor="privacy-redact"
      >
        <Switch
          id="privacy-redact"
          checked={enabled}
          disabled={patch.isPending}
          onCheckedChange={(redactSensitive) => {
            patch.mutate(
              { privacy: { redactSensitive } },
              {
                onError: (error) => toast.error(copy.saveFailed, { description: error.message }),
              },
            );
          }}
        />
      </SettingsRow>
    </SettingsCard>
  );
}

/** "Hide sensitive information", once the settings are loaded. */
export function RedactionSettings() {
  const settings = useSettings();
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {settings.data === undefined ? (
        <SkeletonText lines={1} />
      ) : (
        <RedactionCard enabled={settings.data.privacy.redactSensitive} />
      )}
    </SettingsSection>
  );
}
