import { useState } from 'react';

import { Alert } from '@/ui/alert';
import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { Ellipsis, ICON_PROPS, Pencil, RotateCcw, Trash2, TriangleAlert } from '@/ui/icons';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/ui/menu';

import { disabledCopy, eventCopy, statusCopy, webhooksCopy } from '../copy';
import { useDeleteWebhook, useRotateSecret, useSendTest, useUpdateWebhook } from '../queries';
import type { Webhook } from '../types';
import { DeliveryLog } from './delivery-log';

/** Props of {@link WebhookCard}. */
export interface WebhookCardProps {
  webhook: Webhook;
  onEdit: () => void;
  /** A new secret to show once. */
  onSecret: (secret: string) => void;
}

/** The host of `url`, or the URL itself when it does not parse. */
function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** One endpoint: name, URL, events, its switch, its notice and its actions. */
export function WebhookCard({ webhook, onEdit, onSecret }: WebhookCardProps) {
  const [showLog, setShowLog] = useState(false);
  const update = useUpdateWebhook();
  const toggle = (enabled: boolean) => {
    const { url, label, events, fields } = webhook;
    update.mutate({ id: webhook.id, input: { url, label, events, fields, enabled } });
  };
  return (
    <div className="flex flex-col gap-3 px-4 py-3">
      <CardHeader webhook={webhook} busy={update.isPending} onToggle={toggle} />
      {webhook.disabledReason === null ? null : (
        <Alert variant="warning">
          <TriangleAlert />
          <p>{disabledCopy[webhook.disabledReason]}</p>
        </Alert>
      )}
      <CardActions
        webhook={webhook}
        showLog={showLog}
        onToggleLog={() => {
          setShowLog(!showLog);
        }}
        onEdit={onEdit}
        onSecret={onSecret}
      />
      {showLog ? <DeliveryLog id={webhook.id} /> : null}
    </div>
  );
}

/** Name, URL and events, with the on/off switch. */
function CardHeader(props: { webhook: Webhook; busy: boolean; onToggle: (on: boolean) => void }) {
  const { webhook } = props;
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm text-fg">{webhook.label || hostOf(webhook.url)}</p>
        <p className="mt-0.5 truncate font-mono text-xs text-fg-muted">{webhook.url}</p>
        <p className="mt-1 text-xs text-fg-muted">
          {webhook.events.length === 0
            ? webhooksCopy.noEvents
            : webhook.events.map((event) => eventCopy[event]).join(' · ')}
        </p>
      </div>
      <Switch
        aria-label={webhooksCopy.on}
        checked={webhook.enabled}
        disabled={props.busy}
        onCheckedChange={props.onToggle}
      />
    </div>
  );
}

/** Props of {@link CardActions}. */
interface CardActionsProps extends WebhookCardProps {
  showLog: boolean;
  onToggleLog: () => void;
}

/** Send test (and its outcome), Deliveries, and ⋯ → Edit, New secret, Delete. */
function CardActions({ webhook, showLog, onToggleLog, onEdit, onSecret }: CardActionsProps) {
  const test = useSendTest();
  const result = test.data === undefined ? null : statusCopy[test.data.status].toLowerCase();
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button
        size="sm"
        variant="secondary"
        disabled={test.isPending}
        onClick={() => {
          test.mutate(webhook.id);
        }}
      >
        {test.isPending ? webhooksCopy.sending : webhooksCopy.sendTest}
      </Button>
      <Button size="sm" variant="ghost" aria-expanded={showLog} onClick={onToggleLog}>
        {showLog ? webhooksCopy.hideLog : webhooksCopy.log}
      </Button>
      <span role="status" className="text-xs text-fg-muted">
        {test.error?.message ?? (result === null ? '' : webhooksCopy.tested(result))}
      </span>
      <span className="flex-1" />
      <MoreMenu webhook={webhook} onEdit={onEdit} onSecret={onSecret} />
    </div>
  );
}

/** ⋯ → Edit, New secret, Delete. */
function MoreMenu({ webhook, onEdit, onSecret }: WebhookCardProps) {
  const rotate = useRotateSecret();
  const remove = useDeleteWebhook();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="ghost" size="icon" aria-label={webhooksCopy.edit}>
          <Ellipsis {...ICON_PROPS} />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-44">
        <DropdownMenuItem onSelect={onEdit}>
          <Pencil {...ICON_PROPS} />
          {webhooksCopy.edit}
        </DropdownMenuItem>
        <DropdownMenuItem
          onSelect={() => {
            rotate.mutate(webhook.id, { onSuccess: onSecret });
          }}
        >
          <RotateCcw {...ICON_PROPS} />
          {webhooksCopy.rotate}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          destructive
          onSelect={() => {
            if (window.confirm(webhooksCopy.confirmDelete)) remove.mutate(webhook.id);
          }}
        >
          <Trash2 {...ICON_PROPS} />
          {webhooksCopy.delete}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
