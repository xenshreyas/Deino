/**
 * The planned providers (D12: direct integrations come later): each with
 * what it will do, a quiet WIP badge and a Connect button that cannot be
 * pressed yet. A two-column grid of hairline cells, so seventeen providers
 * stay one glance. Presentational.
 */
import { Button } from '@/ui/button';

import { integrationsCopy } from '../copy';
import { capabilitySummary } from '../format';
import type { ProviderView } from '../types';

const copy = integrationsCopy.planned;

/** Props of {@link PlannedList}. */
export interface PlannedListProps {
  providers: readonly ProviderView[];
}

/** The WIP badge. */
function WipBadge() {
  return (
    <span
      title={copy.badgeTitle}
      className="inline-flex h-5 items-center rounded-sm bg-bg-muted px-1.5 text-xs font-semibold text-fg-muted"
    >
      {copy.badge}
    </span>
  );
}

/** One planned provider. */
function PlannedCell({ provider }: { provider: ProviderView }) {
  return (
    <li className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-3 bg-surface px-4 py-3">
      <div className="min-w-0">
        <p className="flex items-center gap-2 text-sm text-fg">
          {provider.name}
          <WipBadge />
        </p>
        <p className="truncate text-xs text-fg-muted">{capabilitySummary(provider.capabilities)}</p>
      </div>
      <Button
        variant="secondary"
        size="sm"
        disabled
        aria-label={`${integrationsCopy.provider.connect} ${provider.name}`}
      >
        {integrationsCopy.provider.connect}
      </Button>
    </li>
  );
}

/** The planned providers, two to a row on wide windows. */
export function PlannedList({ providers }: PlannedListProps) {
  return (
    <ul
      aria-label={copy.title}
      className="grid gap-px overflow-hidden rounded-lg border border-border bg-border sm:grid-cols-2"
    >
      {providers.map((provider) => (
        <PlannedCell key={provider.id} provider={provider} />
      ))}
      {providers.length % 2 === 1 ? (
        <li aria-hidden className="hidden bg-surface sm:block" />
      ) : null}
    </ul>
  );
}
