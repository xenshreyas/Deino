/**
 * The ElevenLabs key row: a password box and Save until a key is saved,
 * then Check and Forget. The key never comes back from Rust; the row only
 * knows whether one is saved.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Input } from '@/ui/input';
import { SettingsRow } from '@/ui/settings-section';

import { cloudCopy } from '../copy';
import { useCheckElevenLabsKey, useForgetElevenLabsKey, useSaveElevenLabsKey } from '../queries';
import type { SecretStatus } from '../types';

const KEY_INPUT_ID = 'elevenlabs-key';

/** `{ description }` when there is one, so the row's optional prop stays absent otherwise. */
function described(description: string | undefined): { description?: string } {
  return description === undefined ? {} : { description };
}

/** The row for a key in `status`. */
export function KeyRow({ status }: { status: SecretStatus }) {
  return status === 'stored' ? <SavedKey /> : <KeyInput locked={status === 'locked'} />;
}

/** The box and Save. */
function KeyInput({ locked }: { locked: boolean }) {
  const [draft, setDraft] = useState('');
  const save = useSaveElevenLabsKey();
  const note = save.error?.message ?? (locked ? cloudCopy.keyLocked : undefined);
  return (
    <SettingsRow label={cloudCopy.keyLabel} htmlFor={KEY_INPUT_ID} {...described(note)}>
      <div className="flex items-center gap-2">
        <Input
          id={KEY_INPUT_ID}
          type="password"
          autoComplete="off"
          placeholder={cloudCopy.keyPlaceholder}
          className="w-48 font-mono"
          value={draft}
          onChange={(event) => {
            setDraft(event.target.value);
          }}
        />
        <Button
          variant="secondary"
          size="sm"
          disabled={draft.trim() === '' || save.isPending}
          onClick={() => {
            save.mutate(draft.trim(), {
              onSuccess: () => {
                setDraft('');
              },
            });
          }}
        >
          {cloudCopy.save}
        </Button>
      </div>
    </SettingsRow>
  );
}

/** Check and Forget, with the check's answer under the label. */
function SavedKey() {
  const check = useCheckElevenLabsKey();
  const forget = useForgetElevenLabsKey();
  const answer = check.isSuccess ? cloudCopy.keyChecked : check.error?.message;
  const note = forget.error?.message ?? answer ?? cloudCopy.keySaved;
  return (
    <SettingsRow label={cloudCopy.keyLabel} description={note}>
      <div className="flex items-center gap-2">
        <Button
          variant="secondary"
          size="sm"
          disabled={check.isPending}
          onClick={() => {
            check.mutate();
          }}
        >
          {cloudCopy.check}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={forget.isPending}
          onClick={() => {
            forget.mutate();
          }}
        >
          {cloudCopy.forget}
        </Button>
      </div>
    </SettingsRow>
  );
}
