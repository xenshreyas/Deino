/**
 * Settings → People (CAP-04): whether Deino names speakers by voice after
 * each meeting (`Settings::speakers.autoIdentify`), and the voice profiles
 * it names them from (ui.md §3 Settings layout).
 */
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { Switch } from '@/ui/controls';

import { speakersCopy } from '../copy';
import { VoiceProfiles } from './voice-profiles';

/** "Recognise speakers by voice", on by default. */
function IdentifySetting() {
  const settings = useSettings();
  const patch = usePatchSettings();
  const enabled = settings.data?.speakers.autoIdentify ?? true;
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 rounded-lg border border-border bg-surface px-4 py-3">
      <div className="min-w-0">
        <label htmlFor="speakers-auto-identify" className="text-sm text-fg">
          {speakersCopy.settingsIdentifyTitle}
        </label>
        <p className="mt-0.5 text-xs text-pretty text-fg-muted">
          {speakersCopy.settingsIdentifyBody}
        </p>
      </div>
      <Switch
        id="speakers-auto-identify"
        checked={enabled}
        disabled={settings.data === undefined || patch.isPending}
        onCheckedChange={(autoIdentify) => {
          patch.mutate(
            { speakers: { autoIdentify } },
            { onError: (error) => toast.error(error.message) },
          );
        }}
      />
    </div>
  );
}

/** Settings → People. */
export function PeopleSettings() {
  return (
    <div className="flex flex-col gap-8">
      <IdentifySetting />
      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold text-fg">{speakersCopy.profilesTitle}</h2>
        <p className="text-xs text-pretty text-fg-muted">{speakersCopy.profilesBody}</p>
        <div className="pt-2">
          <VoiceProfiles />
        </div>
      </section>
    </div>
  );
}
