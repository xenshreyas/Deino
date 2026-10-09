/**
 * Offline mode as a saved setting (`network.offlineMode`, FND-12). Rust reads
 * it before every request; the Network page shows and changes it.
 */
import { usePatchSettings, useSettings } from './queries';

/** The saved offline mode (`undefined` while the settings load) and its setter. */
export function useOfflineMode(): {
  offlineMode: boolean | undefined;
  setOfflineMode: (offline: boolean) => void;
} {
  const { data } = useSettings();
  const { mutate } = usePatchSettings();
  return {
    offlineMode: data?.network.offlineMode,
    setOfflineMode: (offline) => {
      mutate({ network: { offlineMode: offline } });
    },
  };
}
