/**
 * Settings → Developer → Diagnostics: one switch over
 * `developer.captureCallWindows` (CAP-05). When it is on and speaker tags
 * can't read the meeting app's call window, Rust saves that app's windows,
 * as Accessibility shows them, to Deino's log folder once per meeting.
 */
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';

import { diagnosticsCopy as copy } from '../copy';

const SWITCH_ID = 'developer-capture-call-windows';

/** "Save the call window when speaker names can't be read", off by default. */
export function CallWindowCapture() {
  const settings = useSettings();
  const patch = usePatchSettings();
  const enabled = settings.data?.developer.captureCallWindows ?? false;
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      <SettingsCard>
        <div className="grid grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
          <div className="min-w-0">
            <label htmlFor={SWITCH_ID} className="text-sm text-fg">
              {copy.captureLabel}
            </label>
            <p className="mt-0.5 text-xs text-pretty text-fg-muted">{copy.captureDescription}</p>
          </div>
          <Switch
            id={SWITCH_ID}
            checked={enabled}
            disabled={settings.data === undefined || patch.isPending}
            onCheckedChange={(captureCallWindows) => {
              patch.mutate(
                { developer: { captureCallWindows } },
                {
                  onError: (error) => toast.error(copy.saveFailed, { description: error.message }),
                },
              );
            }}
          />
        </div>
      </SettingsCard>
    </SettingsSection>
  );
}
