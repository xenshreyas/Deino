/**
 * Settings → Integrations (INT-00): the shipped providers with Connect and
 * their accounts, the planned ones as WIP (D12), and what was sent.
 *
 * Offline mode is a plain setting the settings feature owns and passes in;
 * calendars connect in Settings → Calendar, which this page points to.
 */
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { CircleAlert, ICON_PROPS } from '@/ui/icons';
import { SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { integrationsCopy } from '../copy';
import { useDeliveries, useProviders } from '../queries';
import type { ProviderView } from '../types';
import { DeliveryLog } from './delivery-log';
import { PlannedList } from './planned-list';
import { ProviderCard } from './provider-card';

const copy = integrationsCopy;

/** Props of {@link IntegrationsSettings}. */
export interface IntegrationsSettingsProps {
  /** The saved offline mode; `undefined` while the settings load. */
  offlineMode: boolean | undefined;
}

/** The intro, the offline note and the pointer to calendars. */
function Intro({ offlineMode }: IntegrationsSettingsProps) {
  return (
    <div className="flex flex-col gap-3">
      <p className="max-w-prose text-sm text-fg-muted">{copy.intro}</p>
      {offlineMode === true ? (
        <p
          role="status"
          className="flex items-center gap-2 rounded-lg bg-warning-soft px-3 py-2 text-xs text-warning-text"
        >
          <CircleAlert {...ICON_PROPS} size={14} />
          {copy.offline}
        </p>
      ) : null}
      <p className="text-xs text-fg-muted">
        {copy.calendars.text}{' '}
        <Link
          to="/settings"
          search={{ section: 'calendar' }}
          className="text-accent hover:underline"
        >
          {copy.calendars.link}
        </Link>
      </p>
    </div>
  );
}

/** The shipped providers, each in its own card. */
function AvailableSection({ providers, now }: { providers: readonly ProviderView[]; now: Date }) {
  if (providers.length === 0) return null;
  return (
    <SettingsSection title={copy.available.title} description={copy.available.description}>
      <div className="flex flex-col gap-3">
        {providers.map((provider) => (
          <ProviderCard key={provider.id} provider={provider} now={now} />
        ))}
      </div>
    </SettingsSection>
  );
}

/** The providers, split into shipped and planned. */
function ProvidersSections({ now }: { now: Date }) {
  const providers = useProviders();
  if (providers.isPending) return <SkeletonText lines={4} />;
  if (providers.isError) {
    return (
      <p role="alert" className="text-sm text-danger">
        {copy.available.loadError}
      </p>
    );
  }
  const available = providers.data.filter((provider) => provider.available);
  const planned = providers.data.filter((provider) => !provider.available);
  return (
    <>
      <AvailableSection providers={available} now={now} />
      {planned.length === 0 ? null : (
        <SettingsSection title={copy.planned.title} description={copy.planned.description}>
          <PlannedList providers={planned} />
        </SettingsSection>
      )}
    </>
  );
}

/** The delivery log section. */
function DeliveriesSection({ now }: { now: Date }) {
  const deliveries = useDeliveries();
  return (
    <SettingsSection title={copy.deliveries.title} description={copy.deliveries.description}>
      {deliveries.isPending ? <SkeletonText lines={2} /> : null}
      {deliveries.isError ? (
        <p role="alert" className="text-xs text-danger">
          {copy.deliveries.loadError}
        </p>
      ) : null}
      {deliveries.data === undefined ? null : (
        <DeliveryLog deliveries={deliveries.data} now={now} />
      )}
    </SettingsSection>
  );
}

/** The Integrations page. */
export function IntegrationsSettings({ offlineMode }: IntegrationsSettingsProps) {
  // Relative times are read against the moment the page opened.
  const [now] = useState(() => new Date());
  return (
    <div className="flex flex-col gap-8">
      <Intro offlineMode={offlineMode} />
      <ProvidersSections now={now} />
      <DeliveriesSection now={now} />
    </div>
  );
}
