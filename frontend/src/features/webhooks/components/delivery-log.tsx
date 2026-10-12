import { Skeleton } from '@/ui/skeleton';

import { kindLabel, statusCopy, webhooksCopy } from '../copy';
import { useDeliveries } from '../queries';
import type { Delivery, WebhookId } from '../types';

/** A time as the log shows it: "Oct 12, 09:00". */
function when(at: string): string {
  return new Date(at).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** The status colour of a delivery. */
function tone(delivery: Delivery): string {
  switch (delivery.status) {
    case 'succeeded':
      return 'text-success';
    case 'failed':
      return 'text-danger';
    default:
      return 'text-warning-text';
  }
}

/** What happened to a delivery, in a few words. */
function outcome(delivery: Delivery): string {
  const parts = [webhooksCopy.attempts(delivery.attempts)];
  if (delivery.lastStatus !== null) parts.push(`HTTP ${String(delivery.lastStatus)}`);
  else if (delivery.lastError !== null) parts.push(delivery.lastError);
  if (delivery.nextAttemptAt !== null)
    parts.push(webhooksCopy.nextTry(when(delivery.nextAttemptAt)));
  return parts.join(' · ');
}

/** One row of the log. */
function DeliveryRow({ delivery }: { delivery: Delivery }) {
  return (
    <li className="grid grid-cols-[7rem_minmax(0,1fr)_auto] items-baseline gap-3 py-1.5 text-xs">
      <span className="text-fg-muted tabular-nums">{when(delivery.createdAt)}</span>
      <span className="min-w-0 truncate text-fg">
        <span className="font-mono">{kindLabel(delivery.kind)}</span>
        <span className="text-fg-muted"> · {outcome(delivery)}</span>
      </span>
      <span className={tone(delivery)}>{statusCopy[delivery.status]}</span>
    </li>
  );
}

/** An endpoint's newest deliveries, refreshed as attempts land. */
export function DeliveryLog({ id }: { id: WebhookId }) {
  const deliveries = useDeliveries(id);
  if (deliveries.isPending) return <Skeleton className="h-16 w-full" />;
  if (deliveries.isError) {
    return (
      <p role="alert" className="text-xs text-danger">
        {deliveries.error.message}
      </p>
    );
  }
  if (deliveries.data.length === 0) {
    return <p className="text-xs text-fg-muted">{webhooksCopy.noDeliveries}</p>;
  }
  return (
    <ul aria-label={webhooksCopy.log} className="divide-y divide-border">
      {deliveries.data.map((delivery) => (
        <DeliveryRow key={delivery.id} delivery={delivery} />
      ))}
    </ul>
  );
}
