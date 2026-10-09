import { useMountEffect } from '@/hooks/useMountEffect';

import { migrateLegacyLocalStorage } from '../legacy-local-storage';
import { usePatchSettings, useSettingsChanges } from '../queries';

/**
 * Keeps the cached settings equal to the saved ones and, once, moves the
 * old UI's localStorage settings into them. Renders nothing; mount it once,
 * inside the query client provider.
 */
export function SettingsSync() {
  useSettingsChanges();
  const { mutateAsync: patch } = usePatchSettings();
  useMountEffect(() => {
    migrateLegacyLocalStorage(window.localStorage, patch).catch((error: unknown) => {
      // The keys stay in place, so the next launch tries again.
      console.error('[settings] Could not move the legacy localStorage settings', error);
    });
  });
  return null;
}
