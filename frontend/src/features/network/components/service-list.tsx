/**
 * Settings → Advanced → Network services (UX-14): one card per kind, one
 * row per service with its hosts and an on/off switch. Only the services
 * in use show (ones that sent something, or that the user switched off);
 * the full allow-list waits behind "Show all services".
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Switch } from '@/ui/controls';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { SkeletonText } from '@/ui/skeleton';

import { networkCopy, switchLabel } from '../copy';
import { useEgressLog, useServices, useSetServiceEnabled } from '../queries';
import type {
  EgressEntry,
  ServiceId,
  ServiceKind,
  ServiceStatus,
  ServiceSwitchInput,
} from '../types';

/** Kinds in page order. */
const KIND_ORDER: readonly ServiceKind[] = ['llm', 'models', 'integration', 'web'];

/** Props of {@link ServiceGroups}. */
export interface ServiceGroupsProps {
  services: readonly ServiceStatus[];
  onToggle: (input: ServiceSwitchInput) => void;
  /** Switches are disabled while a change is being saved. */
  busy?: boolean;
}

/** One service: its name, its hosts and its switch. */
function ServiceRow({
  service,
  onToggle,
  busy,
}: {
  service: ServiceStatus;
  onToggle: ServiceGroupsProps['onToggle'];
  busy: boolean;
}) {
  const hosts = service.hosts.length > 0 ? service.hosts.join(', ') : networkCopy.noHosts;
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-sm text-fg">{networkCopy.serviceNames[service.id]}</p>
        <p className="truncate font-mono text-xs text-fg-muted" title={hosts}>
          {hosts}
        </p>
      </div>
      <Switch
        aria-label={switchLabel(service.id)}
        checked={service.enabled}
        disabled={busy}
        onCheckedChange={(enabled) => {
          onToggle({ service: service.id, enabled });
        }}
      />
    </div>
  );
}

/** The services grouped by kind, each group a card. */
export function ServiceGroups({ services, onToggle, busy = false }: ServiceGroupsProps) {
  return (
    <div className="flex flex-col gap-4">
      {KIND_ORDER.map((kind) => {
        const members = services.filter((service) => service.kind === kind);
        if (members.length === 0) return null;
        return (
          <div key={kind} className="flex flex-col gap-1.5">
            <h4 className="px-1 text-xs font-semibold text-fg-muted">{networkCopy.kinds[kind]}</h4>
            <SettingsCard label={networkCopy.kinds[kind]}>
              {members.map((service) => (
                <ServiceRow key={service.id} service={service} onToggle={onToggle} busy={busy} />
              ))}
            </SettingsCard>
          </div>
        );
      })}
    </div>
  );
}

/**
 * The services in use: any that appears in the recent activity log, and any
 * the user switched off (so it can be switched back on).
 */
export function servicesInUse(
  services: readonly ServiceStatus[],
  log: readonly EgressEntry[],
): ServiceStatus[] {
  const used = new Set(log.map((entry) => entry.service));
  return services.filter((service) => !service.enabled || used.has(service.id));
}

/**
 * The services in use, or every service once asked for. A service switched
 * here stays listed, so turning one back on never makes it vanish.
 */
function VisibleServices({ services, onToggle, busy = false }: ServiceGroupsProps) {
  const [showAll, setShowAll] = useState(false);
  const [touched, setTouched] = useState<ReadonlySet<ServiceId>>(new Set());
  const log = useEgressLog();
  const inUse = servicesInUse(services, log.data ?? []);
  const shown = showAll
    ? services
    : services.filter((service) => touched.has(service.id) || inUse.includes(service));
  const toggle = (input: ServiceSwitchInput) => {
    setTouched((before) => new Set(before).add(input.service));
    onToggle(input);
  };
  return (
    <>
      {shown.length === 0 ? (
        <p className="px-1 text-sm text-fg-muted">{networkCopy.services.noneInUse}</p>
      ) : (
        <ServiceGroups services={shown} onToggle={toggle} busy={busy} />
      )}
      {showAll || inUse.length === services.length ? null : (
        <Button
          variant="ghost"
          size="sm"
          className="self-start"
          onClick={() => {
            setShowAll(true);
          }}
        >
          {networkCopy.services.showAll(services.length)}
        </Button>
      )}
    </>
  );
}

/** The services section, loaded from the `network` plugin. */
export function ServicesSection() {
  const services = useServices();
  const toggle = useSetServiceEnabled();
  return (
    <SettingsSection
      title={networkCopy.services.title}
      description={networkCopy.services.description}
    >
      {services.isPending ? <SkeletonText lines={4} /> : null}
      {services.isError ? (
        <p role="alert" className="text-sm text-danger">
          {networkCopy.services.error}
        </p>
      ) : null}
      {services.data === undefined ? null : (
        <VisibleServices
          services={services.data}
          onToggle={toggle.mutate}
          busy={toggle.isPending}
        />
      )}
      {toggle.isError ? (
        <p role="alert" className="text-xs text-danger">
          {networkCopy.services.switchError}
        </p>
      ) : null}
    </SettingsSection>
  );
}
