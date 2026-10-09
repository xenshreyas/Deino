import { applyTheme, type ThemePreference } from '@/features/appearance';
import { useMountEffect } from '@/hooks/useMountEffect';

import { migrateLegacyLocalStorage } from '../legacy-local-storage';
import { usePatchSettings, useSettings, useSettingsChanges } from '../queries';

/** Puts the saved theme on screen; remounted (by `key`) whenever it changes. */
function SavedTheme({ theme }: { theme: ThemePreference }) {
  useMountEffect(() => {
    applyTheme(theme);
  });
  return null;
}

/**
 * Keeps the cached settings equal to the saved ones, applies the saved theme
 * (including changes saved elsewhere) and, once, moves the old UI's
 * localStorage settings into them. Renders nothing; mount it once, inside
 * the query client provider.
 */
export function SettingsSync() {
  useSettingsChanges();
  const { data: settings } = useSettings();
  const { mutateAsync: patch } = usePatchSettings();
  useMountEffect(() => {
    migrateLegacyLocalStorage(window.localStorage, patch).catch((error: unknown) => {
      // The keys stay in place, so the next launch tries again.
      console.error('[settings] Could not move the legacy localStorage settings', error);
    });
  });
  const theme = settings?.ui.theme;
  return theme === undefined ? null : <SavedTheme key={theme} theme={theme} />;
}
