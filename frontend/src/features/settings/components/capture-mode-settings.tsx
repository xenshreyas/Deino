/**
 * Settings → Transcription → While you record (CAP-08): live transcription
 * on or off, and the high-accuracy re-pass after each meeting. Values live
 * in the typed settings (`transcription.liveTranscription`,
 * `transcription.highAccuracyRepass`); the recorder reads them at every
 * start and the `final_transcription` stage after every meeting.
 */
import { toast } from 'sonner';

import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { settingsCopy } from '../copy';
import { usePatchSettings, useSettings } from '../queries';
import type { Settings } from '../types';

const copy = settingsCopy.captureModes;

/** Saves a change to the transcription section. */
function useSaveTranscription() {
  const patch = usePatchSettings();
  const save = (change: Partial<Settings['transcription']>) => {
    patch.mutate(
      { transcription: change },
      { onError: (error) => toast.error(copy.saveFailed, { description: error.message }) },
    );
  };
  return { save, pending: patch.isPending };
}

/** The two switches over the saved `transcription` section. */
function CaptureModeCard({ transcription }: { transcription: Settings['transcription'] }) {
  const { save, pending } = useSaveTranscription();
  const live = transcription.liveTranscription;
  return (
    <SettingsCard label={copy.title}>
      <SettingsRow
        label={copy.liveTitle}
        description={live ? copy.liveOn : copy.liveOff}
        htmlFor="transcription-live"
      >
        <Switch
          id="transcription-live"
          checked={live}
          disabled={pending}
          onCheckedChange={(liveTranscription) => {
            save({ liveTranscription });
          }}
        />
      </SettingsRow>
      <SettingsRow
        label={copy.repassTitle}
        description={live ? copy.repassOn : copy.repassOff}
        htmlFor="transcription-repass"
      >
        <Switch
          id="transcription-repass"
          checked={live && transcription.highAccuracyRepass}
          disabled={pending || !live}
          onCheckedChange={(highAccuracyRepass) => {
            save({ highAccuracyRepass });
          }}
        />
      </SettingsRow>
    </SettingsCard>
  );
}

/** Live transcription and the re-pass, once the settings are loaded. */
export function CaptureModeSettings() {
  const settings = useSettings();
  return (
    <SettingsSection title={copy.title} description={copy.description}>
      {settings.data === undefined ? (
        <SkeletonText lines={2} />
      ) : (
        <CaptureModeCard transcription={settings.data.transcription} />
      )}
    </SettingsSection>
  );
}
