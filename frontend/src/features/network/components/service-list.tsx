/**
 * The services on the Network page: one card per kind, one row per service
 * with its hosts and an on/off switch.
 */
import { SkeletonText } from '@/ui/skeleton';
import { Switch } from '@/ui/controls';

import { networkCopy, switchLabel } from '../copy';
import { useServices, useSetServiceEnabled } from '../queries';
import type { ServiceKind, ServiceStatus, ServiceSwitchInput } from '../types';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';

/** Kinds in page order. */
const KIND_ORDER: readonly ServiceKind[] = ['llm', 'models', 'integration', 'web'];

/** Props of {@link ServiceGroups}. */
export interface ServiceGroupsProps {
  services: readonly ServiceStatus[];
  onToggle: (input: ServiceSwitchInput) => void;
  /** Switches are disabled while a change is being saved. */
  busy?: boolean;
}

/** What `service` receives, when the Network page says so (CAP-14: audio). */
function serviceNote(service: ServiceStatus['id']): string | undefined {
  const notes: Partial<Record<ServiceStatus['id'], string>> = networkCopy.serviceNotes;
  return notes[service];
}

/** One service: its name, what it receives, its hosts and its switch. */
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
  const note = serviceNote(service.id);
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-sm text-fg">{networkCopy.serviceNames[service.id]}</p>
        {note === undefined ? null : <p className="text-xs text-fg-muted">{note}</p>}
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
        <ServiceGroups services={services.data} onToggle={toggle.mutate} busy={toggle.isPending} />
      )}
      {toggle.isError ? (
        <p role="alert" className="text-xs text-danger">
          {networkCopy.services.switchError}
        </p>
      ) : null}
    </SettingsSection>
  );
}
