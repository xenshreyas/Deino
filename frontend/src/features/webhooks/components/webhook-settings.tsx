import { useState } from 'react';

import { Button } from '@/ui/button';
import { ICON_PROPS, Plus } from '@/ui/icons';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { Skeleton } from '@/ui/skeleton';

import { webhooksCopy } from '../copy';
import { useCreateWebhook, useUpdateWebhook, useWebhooks } from '../queries';
import type { Webhook, WebhookId } from '../types';
import { SecretNotice } from './secret-notice';
import { WebhookCard } from './webhook-card';
import { WebhookForm } from './webhook-form';

/** Which form is open: none, a new endpoint, or an endpoint's edit. */
type Editing = { kind: 'none' } | { kind: 'new' } | { kind: 'edit'; id: WebhookId };

/**
 * Settings → Developer → Webhooks: the endpoints, adding and editing one,
 * its signing secret once, test sends and the delivery log.
 */
export function WebhookSettings() {
  const webhooks = useWebhooks();
  const [editing, setEditing] = useState<Editing>({ kind: 'none' });
  const [secret, setSecret] = useState<string | null>(null);
  return (
    <SettingsSection title={webhooksCopy.title} description={webhooksCopy.description}>
      {secret === null ? null : (
        <SecretNotice
          secret={secret}
          onDismiss={() => {
            setSecret(null);
          }}
        />
      )}
      {webhooks.isPending ? <Skeleton className="h-24 w-full" /> : null}
      {webhooks.isError ? (
        <p role="alert" className="text-sm text-danger">
          {webhooksCopy.loadFailed}
        </p>
      ) : null}
      {webhooks.data === undefined ? null : (
        <WebhookList
          webhooks={webhooks.data}
          editing={editing}
          onEditing={setEditing}
          onSecret={setSecret}
        />
      )}
    </SettingsSection>
  );
}

/** Props of {@link WebhookList}. */
interface WebhookListProps {
  webhooks: Webhook[];
  editing: Editing;
  onEditing: (editing: Editing) => void;
  onSecret: (secret: string) => void;
}

/** The endpoints, each as a card or its edit form, then the add form or button. */
function WebhookList({ webhooks, editing, onEditing, onSecret }: WebhookListProps) {
  const close = () => {
    onEditing({ kind: 'none' });
  };
  return (
    <>
      {webhooks.length === 0 && editing.kind !== 'new' ? <EmptyCard /> : null}
      {webhooks.length === 0 ? null : (
        <SettingsCard label={webhooksCopy.title}>
          {webhooks.map((webhook) =>
            editing.kind === 'edit' && editing.id === webhook.id ? (
              <EditForm key={webhook.id} webhook={webhook} onDone={close} />
            ) : (
              <WebhookCard
                key={webhook.id}
                webhook={webhook}
                onSecret={onSecret}
                onEdit={() => {
                  onEditing({ kind: 'edit', id: webhook.id });
                }}
              />
            ),
          )}
        </SettingsCard>
      )}
      {editing.kind === 'new' ? (
        <NewForm onDone={close} onSecret={onSecret} />
      ) : (
        <AddButton
          onClick={() => {
            onEditing({ kind: 'new' });
          }}
        />
      )}
    </>
  );
}

/** "Add webhook". */
function AddButton({ onClick }: { onClick: () => void }) {
  return (
    <div>
      <Button size="sm" variant="secondary" onClick={onClick}>
        <Plus {...ICON_PROPS} />
        {webhooksCopy.add}
      </Button>
    </div>
  );
}

/** The quiet card shown before the first endpoint. */
function EmptyCard() {
  return (
    <SettingsCard label={webhooksCopy.title}>
      <div className="px-4 py-3">
        <p className="text-sm text-fg">{webhooksCopy.empty}</p>
        <p className="mt-0.5 text-xs text-fg-muted">{webhooksCopy.emptyHint}</p>
      </div>
    </SettingsCard>
  );
}

/** Adds an endpoint and shows its secret. */
function NewForm({ onDone, onSecret }: { onDone: () => void; onSecret: (s: string) => void }) {
  const create = useCreateWebhook();
  return (
    <SettingsCard label={webhooksCopy.add}>
      <WebhookForm
        busy={create.isPending}
        error={create.error}
        onCancel={onDone}
        onSubmit={(input) => {
          create.mutate(input, {
            onSuccess: (created) => {
              onSecret(created.secret);
              onDone();
            },
          });
        }}
      />
    </SettingsCard>
  );
}

/** Edits an endpoint in place. */
function EditForm({ webhook, onDone }: { webhook: Webhook; onDone: () => void }) {
  const update = useUpdateWebhook();
  return (
    <WebhookForm
      webhook={webhook}
      busy={update.isPending}
      error={update.error}
      onCancel={onDone}
      onSubmit={(input) => {
        update.mutate({ id: webhook.id, input }, { onSuccess: onDone });
      }}
    />
  );
}
