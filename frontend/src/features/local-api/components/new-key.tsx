import type { UseMutationResult } from '@tanstack/react-query';
import { type FormEvent, useState } from 'react';

import type { IpcError } from '@/lib/ipc/result';
import { Button } from '@/ui/button';
import { Check, Copy } from '@/ui/icons';
import { Input } from '@/ui/input';
import { SegmentedControl } from '@/ui/segmented-control';

import { apiCopy } from '../copy';
import type { ApiKeyScope, CreatedApiKey, NewApiKey } from '../types';

const SCOPES = [
  { value: 'read', label: apiCopy.scopes.read },
  { value: 'read_write', label: apiCopy.scopes.read_write },
] as const;

/** Props of {@link NewKeyForm}. */
export interface NewKeyFormProps {
  create: UseMutationResult<CreatedApiKey, IpcError, NewApiKey>;
}

/** A name and an access level, then Create. */
export function NewKeyForm({ create }: NewKeyFormProps) {
  const [name, setName] = useState('');
  const [scope, setScope] = useState<ApiKeyScope>('read');
  const submit = (event: FormEvent) => {
    event.preventDefault();
    create.mutate({ name, scope });
  };
  return (
    <form onSubmit={submit} className="flex flex-col gap-2 px-4 py-3">
      <div className="flex flex-wrap items-center gap-2">
        <Input
          aria-label={apiCopy.name}
          placeholder={apiCopy.namePlaceholder}
          className="min-w-0 flex-1"
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
        <SegmentedControl
          label={apiCopy.scope}
          segments={SCOPES}
          value={scope}
          onValueChange={setScope}
        />
        <Button type="submit" size="sm" disabled={create.isPending || name.trim() === ''}>
          {create.isPending ? apiCopy.creating : apiCopy.create}
        </Button>
      </div>
      {create.error === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {create.error.message}
        </p>
      )}
    </form>
  );
}

/** Props of {@link NewKeyReveal}. */
export interface NewKeyRevealProps {
  created: CreatedApiKey;
  onDone: () => void;
}

/** The new key's token, shown this once, with a copy button. */
export function NewKeyReveal({ created, onDone }: NewKeyRevealProps) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard.writeText(created.token).then(() => {
      setCopied(true);
    });
  };
  return (
    <div className="flex flex-col gap-2 px-4 py-3">
      <div>
        <p className="text-sm text-fg">{apiCopy.newKey}</p>
        <p className="mt-0.5 text-xs text-fg-muted">{apiCopy.newKeyHint}</p>
      </div>
      <pre className="overflow-x-auto rounded-md bg-bg-muted px-3 py-2 font-mono text-xs text-fg select-all">
        {created.token}
      </pre>
      <div className="flex justify-end gap-2">
        <Button variant="secondary" size="sm" onClick={copy}>
          {copied ? <Check /> : <Copy />}
          {copied ? apiCopy.copied : apiCopy.copy}
        </Button>
        <Button size="sm" onClick={onDone}>
          {apiCopy.done}
        </Button>
      </div>
    </div>
  );
}
