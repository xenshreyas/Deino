/**
 * Settings → Privacy → Network (FND-12): offline mode and what has left the
 * Mac. Which services may send is Settings → Advanced (UX-14,
 * {@link ServicesSection}).
 *
 * Offline mode is a plain setting (`network.offlineMode`), so the settings
 * feature owns its state and passes it in; this feature owns the services
 * and the egress log.
 */
import { Switch } from '@/ui/controls';

import { SettingsCard, SettingsSection } from '@/ui/settings-section';

import { networkCopy } from '../copy';
import { EgressLogSection } from './egress-log';

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

/** Offline mode, then what left this Mac. */
export function NetworkSettings(props: NetworkSettingsProps) {
  return (
    <div className="flex flex-col gap-8">
      <SettingsSection
        title={networkCopy.offline.sectionTitle}
        description={networkCopy.offline.sectionDescription}
      >
        <OfflineModeCard {...props} />
      </SettingsSection>
      <EgressLogSection />
    </div>
  );
}
