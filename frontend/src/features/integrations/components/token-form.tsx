/**
 * The token paste form, inline under its provider: the token (write-only,
 * it goes to the Keychain), the username for providers that pair one with
 * it, and Connect. Presentational.
 */
import { type FormEvent, useId, useState } from 'react';

import { Button } from '@/ui/button';
import { Input } from '@/ui/input';

import { integrationsCopy } from '../copy';
import type { ConnectMethod } from '../types';

const copy = integrationsCopy.provider;

/** What the form submits. */
export interface TokenFields {
  token: string;
  username: string | null;
}

/** Props of {@link TokenForm}. */
export interface TokenFormProps {
  method: Extract<ConnectMethod, { method: 'token' }>;
  busy: boolean;
  /** Why the last attempt failed, user-safe. */
  error: string | null;
  onSubmit: (fields: TokenFields) => void;
  onCancel: () => void;
}

/** One labelled field. */
function Field(props: {
  id: string;
  label: string;
  type: 'text' | 'password';
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={props.id} className="text-xs text-fg-muted">
        {props.label}
      </label>
      <Input
        id={props.id}
        type={props.type}
        value={props.value}
        autoComplete="off"
        spellCheck={false}
        onChange={(event) => {
          props.onChange(event.currentTarget.value);
        }}
      />
    </div>
  );
}

/** The inline form. */
export function TokenForm({ method, busy, error, onSubmit, onCancel }: TokenFormProps) {
  const id = useId();
  const [token, setToken] = useState('');
  const [username, setUsername] = useState('');
  const needsUsername = method.usernameLabel !== null;
  const ready = token.trim() !== '' && (!needsUsername || username.trim() !== '');
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (ready) onSubmit({ token, username: needsUsername ? username : null });
  };
  return (
    <form onSubmit={submit} className="flex flex-col gap-3 px-4 py-3">
      {method.usernameLabel === null ? null : (
        <Field
          id={`${id}-username`}
          label={method.usernameLabel}
          type="text"
          value={username}
          onChange={setUsername}
        />
      )}
      <Field
        id={`${id}-token`}
        label={method.label}
        type="password"
        value={token}
        onChange={setToken}
      />
      {error === null ? null : (
        <p role="alert" className="text-xs text-danger">
          {error}
        </p>
      )}
      <Actions canSubmit={!busy && ready} onCancel={onCancel} />
    </form>
  );
}

/** Cancel and Connect, right-aligned. */
function Actions({ canSubmit, onCancel }: { canSubmit: boolean; onCancel: () => void }) {
  return (
    <div className="flex justify-end gap-2">
      <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
        {copy.cancel}
      </Button>
      <Button type="submit" size="sm" disabled={!canSubmit}>
        {copy.save}
      </Button>
    </div>
  );
}
