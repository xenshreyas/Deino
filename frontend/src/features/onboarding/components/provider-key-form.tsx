import { type FormEvent, useState } from 'react';

import { useSecretStatus } from '@/features/settings';
import { Button } from '@/ui/button';
import { CircleCheck, ICON_PROPS } from '@/ui/icons';
import { Input } from '@/ui/input';

import { KEYED_PROVIDERS, type KeyedChoice } from '../ai-choice';
import { aiCopy } from '../copy';
import { useConnectProvider } from '../hooks/use-connect-provider';

/**
 * Paste a Claude or OpenAI key: it is checked through the LLM gateway
 * before it is saved, and once saved only its status is ever shown.
 */
export function ProviderKeyForm({ choice }: { choice: KeyedChoice }) {
  const [key, setKey] = useState('');
  const connect = useConnectProvider();
  const saved = useSecretStatus(KEYED_PROVIDERS[choice].secret).data === 'stored';
  const submit = (event: FormEvent) => {
    event.preventDefault();
    connect.mutate(
      { choice, key: key.trim() },
      {
        onSuccess: () => {
          setKey('');
        },
      },
    );
  };
  const inputId = `onboarding-key-${choice}`;
  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <label htmlFor={inputId} className="text-sm font-semibold text-fg">
        {aiCopy.keyLabel(aiCopy.choices[choice])}
      </label>
      <div className="flex gap-2">
        <KeyInput
          id={inputId}
          value={key}
          saved={saved}
          invalid={connect.isError}
          onChange={setKey}
        />
        <Button type="submit" variant="secondary" disabled={key.trim() === '' || connect.isPending}>
          {connect.isPending ? aiCopy.checking : aiCopy.check}
        </Button>
      </div>
      <KeyStatus saved={saved} rejected={connect.isError} />
    </form>
  );
}

interface KeyInputProps {
  id: string;
  value: string;
  saved: boolean;
  invalid: boolean;
  onChange: (value: string) => void;
}

/** The masked key field; a saved key shows as dots, never as text. */
function KeyInput({ id, value, saved, invalid, onChange }: KeyInputProps) {
  return (
    <Input
      id={id}
      type="password"
      autoComplete="off"
      spellCheck={false}
      value={value}
      placeholder={saved ? '••••••••••••' : ''}
      aria-invalid={invalid}
      onChange={(event) => {
        onChange(event.target.value);
      }}
      className="flex-1"
    />
  );
}

/** "Key saved", the rejection, or the Keychain note. */
function KeyStatus({ saved, rejected }: { saved: boolean; rejected: boolean }) {
  if (rejected) {
    return (
      <p role="alert" className="text-xs text-danger">
        {aiCopy.keyRejected}
      </p>
    );
  }
  if (saved) {
    return (
      <p className="flex items-center gap-1.5 text-xs text-fg-muted">
        <CircleCheck {...ICON_PROPS} className="text-accent" />
        {aiCopy.keySaved}
      </p>
    );
  }
  return <p className="text-xs text-fg-subtle">{aiCopy.keyHelp}</p>;
}
