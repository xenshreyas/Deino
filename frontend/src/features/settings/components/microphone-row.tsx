/**
 * The microphone recordings start with (`audio.preferredMicDevice`; empty
 * means the Mac's default), picked from the Mac's microphones. Locked while
 * a recording runs: the record panel switches microphones mid-call.
 */
import { useQuery } from '@tanstack/react-query';

import { recordingApi, recordingKeys, useRecordingState } from '@/features/recording';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';
import { SettingsRow } from '@/ui/settings-section';

import { recordingSettingsCopy as copy } from '../copy';
import { useSaveSettings } from './save-settings';

/** The select's value for "the Mac's default". */
const DEFAULT = '__default__';

/** The Mac's microphones, default first. */
function useMicrophones() {
  return useQuery({
    queryKey: [...recordingKeys.all, 'microphones'],
    queryFn: recordingApi.microphones,
  });
}

/** Props of {@link MicrophoneRow}. */
export interface MicrophoneRowProps {
  /** The saved microphone; `null` for the Mac's default. */
  preferred: string | null;
}

/** "Microphone" and its select. */
export function MicrophoneRow({ preferred }: MicrophoneRowProps) {
  const microphones = useMicrophones();
  const { save, saving } = useSaveSettings();
  const recording = useRecordingState().phase !== 'idle';
  const names = (microphones.data ?? []).map((microphone) => microphone.name);
  const known = preferred === null || names.includes(preferred) ? names : [...names, preferred];
  return (
    <SettingsRow
      label={copy.microphoneTitle}
      description={recording ? copy.microphoneLocked : copy.microphoneDescription}
    >
      <Select
        value={preferred ?? DEFAULT}
        disabled={recording || saving}
        onValueChange={(value) => {
          const name = value === DEFAULT ? null : value;
          save({ audio: { preferredMicDevice: name } });
        }}
      >
        <SelectTrigger aria-label={copy.microphoneTitle} className="w-[220px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={DEFAULT}>{copy.microphoneDefault}</SelectItem>
          {known.map((name) => (
            <SelectItem key={name} value={name}>
              {name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </SettingsRow>
  );
}
