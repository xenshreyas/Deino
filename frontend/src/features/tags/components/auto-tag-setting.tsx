/**
 * The auto-tagging switch (`Settings::organize.autoTag`), as a settings row
 * card at the top of the Tags page (ui.md §3 Settings layout: setting row).
 */
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { Switch } from '@/ui/controls';

import { tagsCopy } from '../copy';

/** "Auto-tag new meetings", on by default. */
export function AutoTagSetting() {
  const settings = useSettings();
  const patch = usePatchSettings();
  const enabled = settings.data?.organize.autoTag ?? true;
  return (
    <div className="grid grid-cols-[1fr_auto] items-center gap-4 rounded-lg border border-border bg-surface px-4 py-3">
      <div className="min-w-0">
        <label htmlFor="auto-tag" className="text-sm text-fg">
          {tagsCopy.autoTagTitle}
        </label>
        <p className="mt-0.5 text-xs text-pretty text-fg-muted">{tagsCopy.autoTagDescription}</p>
      </div>
      <Switch
        id="auto-tag"
        checked={enabled}
        disabled={settings.data === undefined || patch.isPending}
        onCheckedChange={(autoTag) => {
          patch.mutate(
            { organize: { autoTag } },
            {
              onError: (error) => {
                toast.error(error.message);
              },
            },
          );
        }}
      />
    </div>
  );
}
