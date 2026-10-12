/**
 * Settings → Transcription → ElevenLabs (optional) (CAP-14): one card with
 * the key, the model and the switch. The switch and the model are typed
 * settings (`transcription.cloudProvider`, `transcription.cloudModel`); the
 * `final_transcription` step reads them after every meeting. The switch
 * stays off and disabled until a key is saved.
 */
import { toast } from 'sonner';

import { type Settings, usePatchSettings, useSettings } from '@/features/settings';
import { Switch } from '@/ui/controls';
import { type Option, OptionSelect } from '@/ui/option-select';
import { SettingsCard, SettingsRow, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { cloudCopy } from '../copy';
import { useElevenLabsKey } from '../queries';
import type { CloudTranscriptionModel, SecretStatus } from '../types';
import { KeyRow } from './key-row';

const SWITCH_ID = 'elevenlabs-after-meeting';

const MODEL_OPTIONS: readonly Option<CloudTranscriptionModel>[] = [
  { value: 'scribe_v2', label: cloudCopy.models.scribe_v2 },
  { value: 'scribe_v1', label: cloudCopy.models.scribe_v1 },
];

type Transcription = Settings['transcription'];

/** Saves a change to the transcription section's cloud fields. */
function useSaveCloudOption() {
  const patch = usePatchSettings();
  const save = (change: Partial<Transcription>) => {
    patch.mutate(
      { transcription: change },
      { onError: (error) => toast.error(cloudCopy.saveFailed, { description: error.message }) },
    );
  };
  return { save, pending: patch.isPending };
}

/** Props of {@link ElevenLabsCard}. */
interface ElevenLabsCardProps {
  transcription: Transcription;
  keyStatus: SecretStatus;
}

/** The key, the model and the switch. */
export function ElevenLabsCard({ transcription, keyStatus }: ElevenLabsCardProps) {
  const { save, pending } = useSaveCloudOption();
  const stored = keyStatus === 'stored';
  const needsKey = stored ? {} : { description: cloudCopy.switchNeedsKey };
  return (
    <SettingsCard label={cloudCopy.card}>
      <KeyRow status={keyStatus} />
      <SettingsRow label={cloudCopy.modelLabel}>
        <OptionSelect
          label={cloudCopy.modelLabel}
          value={transcription.cloudModel}
          options={MODEL_OPTIONS}
          onChange={(cloudModel) => {
            save({ cloudModel });
          }}
        />
      </SettingsRow>
      <SettingsRow label={cloudCopy.switchLabel} htmlFor={SWITCH_ID} {...needsKey}>
        <Switch
          id={SWITCH_ID}
          checked={stored && transcription.cloudProvider === 'elevenlabs'}
          disabled={pending || !stored}
          onCheckedChange={(on) => {
            save({ cloudProvider: on ? 'elevenlabs' : 'none' });
          }}
        />
      </SettingsRow>
    </SettingsCard>
  );
}

/** The section, once the settings and the key's status are loaded. */
export function ElevenLabsSettings() {
  const settings = useSettings();
  const key = useElevenLabsKey();
  return (
    <SettingsSection title={cloudCopy.title} description={cloudCopy.description}>
      {settings.data === undefined || key.data === undefined ? (
        <SkeletonText lines={3} />
      ) : (
        <ElevenLabsCard transcription={settings.data.transcription} keyStatus={key.data} />
      )}
    </SettingsSection>
  );
}
