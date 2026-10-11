/**
 * Settings → Privacy → Network (FND-12): offline mode, which services may
 * send, and what has left the Mac.
 *
 * Offline mode is a plain setting (`network.offlineMode`), so the settings
 * feature owns its state and passes it in; this feature owns the services
 * and the egress log.
 */
import { Switch } from '@/ui/controls';

import { networkCopy } from '../copy';
import { EgressLogSection } from './egress-log';
import { ServicesSection } from './service-list';
import { SettingsCard } from '@/ui/settings-section';

/** Props of {@link NetworkSettings}. */
export interface NetworkSettingsProps {
  /** The saved offline mode; `undefined` while the settings load. */
  offlineMode: boolean | undefined;
  onOfflineModeChange: (offline: boolean) => void;
}

/** The offline mode switch, in its own card. */
function OfflineModeCard({ offlineMode, onOfflineModeChange }: NetworkSettingsProps) {
  return (
    <SettingsCard>
      <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
        <div>
          <p id="offline-mode-label" className="text-sm text-fg">
            {networkCopy.offline.title}
          </p>
          <p className="text-xs text-fg-muted">{networkCopy.offline.description}</p>
        </div>
        <Switch
          aria-labelledby="offline-mode-label"
          checked={offlineMode ?? false}
          disabled={offlineMode === undefined}
          onCheckedChange={onOfflineModeChange}
        />
      </div>
    </SettingsCard>
  );
}

/** The Network page. */
export function NetworkSettings(props: NetworkSettingsProps) {
  return (
    <div className="flex flex-col gap-8">
      <OfflineModeCard {...props} />
      <ServicesSection />
      <EgressLogSection />
    </div>
  );
}
