/**
 * Speakers in Settings (CAP-04): Recording → "Recognise speakers by voice"
 * after each meeting (`Settings::speakers.autoIdentify`), and Advanced →
 * Voices, the voice profiles it names them from (UX-18).
 */
import { toast } from 'sonner';

import { usePatchSettings, useSettings } from '@/features/settings';
import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';

import { speakersCopy } from '../copy';
import { VoiceProfiles } from './voice-profiles';

/** "Recognise speakers by voice", on by default. */
function IdentifySetting() {
  const settings = useSettings();
  const patch = usePatchSettings();
  const enabled = settings.data?.speakers.autoIdentify ?? true;
  return (
    <SettingsCard label={speakersCopy.settingsIdentifyTitle}>
      <SettingsRow
        label={speakersCopy.settingsIdentifyTitle}
        description={speakersCopy.settingsIdentifyBody}
        htmlFor="speakers-auto-identify"
      >
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
      </SettingsRow>
    </SettingsCard>
  );
}

/** Settings → Recording → Speakers: the voice switch. */
export function SpeakerSettings() {
  return (
    <SettingsSection title={speakersCopy.settingsTitle} description={speakersCopy.settingsBody}>
      <IdentifySetting />
    </SettingsSection>
  );
}

/** Settings → Advanced → Voices: the voice profiles. */
export function VoiceSettings() {
  return (
    <SettingsSection title={speakersCopy.profilesTitle} description={speakersCopy.profilesBody}>
      <VoiceProfiles />
    </SettingsSection>
  );
}
