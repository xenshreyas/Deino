/**
 * What Deino sent to integrations: provider, what it wrote, when, whether it
 * went through, a link to what it created, and why it failed. Presentational.
 */
import { cn } from '@/ui/cn';
import { SettingsCard } from '@/ui/settings-section';

import { integrationsCopy } from '../copy';
import { relativeTime } from '../format';
import type { Delivery } from '../types';

const copy = integrationsCopy.deliveries;

/** Props of {@link DeliveryLog}. */
export interface DeliveryLogProps {
  deliveries: readonly Delivery[];
  now: Date;
}

/** The status word, coloured only when it needs attention. */
function StatusWord({ delivery }: { delivery: Delivery }) {
  return (
    <span className={cn(delivery.status === 'failed' ? 'text-danger' : 'text-fg-muted')}>
      {integrationsCopy.deliveryStatuses[delivery.status]}
    </span>
  );
}

/** One delivery. */
function DeliveryRow({ delivery, now }: { delivery: Delivery; now: Date }) {
  const url = delivery.external?.url ?? null;
  return (
    <li className="px-4 py-2.5">
      <div className="flex items-baseline justify-between gap-4 text-sm">
        <p className="min-w-0 truncate text-fg">
          {integrationsCopy.providerNames[delivery.provider]}
          <span className="text-fg-muted">
            {' · '}
            {integrationsCopy.capabilities[delivery.capability]}
          </span>
        </p>
        <p className="flex shrink-0 items-baseline gap-2 text-xs tabular-nums">
          <StatusWord delivery={delivery} />
          <span className="text-fg-muted">{relativeTime(delivery.createdAt, now)}</span>
          {url === null ? null : (
            <a href={url} target="_blank" rel="noreferrer" className="text-accent hover:underline">
              {copy.open}
            </a>
          )}
        </p>
      </div>
      {delivery.error === null ? null : (
        <p className="mt-0.5 text-xs text-danger">
          {delivery.error.message} · {copy.attempts(delivery.attempts)}
        </p>
      )}
    </li>
  );
}

/** The newest deliveries, or a quiet empty line. */
export function DeliveryLog({ deliveries, now }: DeliveryLogProps) {
  if (deliveries.length === 0) {
    return <p className="text-xs text-fg-muted">{copy.empty}</p>;
  }
  return (
    <SettingsCard label={copy.title}>
      <ul className="divide-y divide-border">
        {deliveries.map((delivery) => (
          <DeliveryRow key={delivery.id} delivery={delivery} now={now} />
        ))}
      </ul>
    </SettingsCard>
  );
}
