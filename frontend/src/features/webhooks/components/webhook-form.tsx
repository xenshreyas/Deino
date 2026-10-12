import { type FormEvent, useState } from 'react';

import type { IpcError } from '@/lib/ipc/result';
import { Button } from '@/ui/button';
import { Input } from '@/ui/input';

import { webhooksCopy } from '../copy';
import { DEFAULT_PAYLOAD, type Webhook, type WebhookInput } from '../types';
import { EventPicker, FieldPicker } from './checkbox-group';

/** Props of {@link WebhookForm}. */
export interface WebhookFormProps {
  /** The endpoint to edit; a new one when absent. */
  webhook?: Webhook;
  busy: boolean;
  error: IpcError | null;
  onSubmit: (input: WebhookInput) => void;
  onCancel: () => void;
}

/** The form's starting values. */
function initial(webhook: Webhook | undefined): WebhookInput {
  if (webhook === undefined) {
    return {
      url: '',
      label: '',
      events: ['meeting.ready'],
      fields: DEFAULT_PAYLOAD,
      enabled: true,
    };
  }
  const { url, label, events, fields, enabled } = webhook;
  return { url, label, events, fields, enabled };
}

/** The URL and name inputs. */
function UrlAndName({
  input,
  onChange,
}: {
  input: WebhookInput;
  onChange: (i: WebhookInput) => void;
}) {
  return (
    <div className="grid gap-3 sm:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
      <Input
        aria-label={webhooksCopy.url}
        placeholder={webhooksCopy.urlPlaceholder}
        className="font-mono"
        value={input.url}
        required
        onChange={(event) => {
          onChange({ ...input, url: event.target.value });
        }}
      />
      <Input
        aria-label={webhooksCopy.name}
        placeholder={webhooksCopy.namePlaceholder}
        value={input.label}
        onChange={(event) => {
          onChange({ ...input, label: event.target.value });
        }}
      />
    </div>
  );
}

/** Adds or edits an endpoint: URL, name, events and payload fields. */
export function WebhookForm({ webhook, busy, error, onSubmit, onCancel }: WebhookFormProps) {
  const [input, setInput] = useState<WebhookInput>(() => initial(webhook));
  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSubmit(input);
  };
  return (
    <form className="flex flex-col gap-4 px-4 py-4" onSubmit={submit}>
      <UrlAndName input={input} onChange={setInput} />
      <EventPicker
        events={input.events}
        onChange={(events) => {
          setInput({ ...input, events });
        }}
      />
      <FieldPicker
        fields={input.fields}
        onChange={(fields) => {
          setInput({ ...input, fields });
        }}
      />
      <FormFooter
        error={error}
        busy={busy || input.url.trim() === ''}
        isNew={webhook === undefined}
        onCancel={onCancel}
      />
    </form>
  );
}

/** The form's error, Cancel and Add or Save. */
function FormFooter(props: {
  error: IpcError | null;
  busy: boolean;
  isNew: boolean;
  onCancel: () => void;
}) {
  const { error } = props;
  return (
    <>
      {error === null ? null : (
        <p role="alert" className="text-sm text-danger">
          {error.message}
        </p>
      )}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={props.onCancel}>
          {webhooksCopy.cancel}
        </Button>
        <Button type="submit" disabled={props.busy}>
          {props.isNew ? webhooksCopy.create : webhooksCopy.save}
        </Button>
      </div>
    </>
  );
}
