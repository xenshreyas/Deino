/**
 * The theme as a saved setting (`ui.theme`).
 *
 * The appearance feature puts a theme on screen; this module saves the
 * user's choice in the typed settings store and reads it back. The saved
 * value wins over the first-paint cache once the settings have loaded
 * (`SettingsSync` applies it).
 */
import { applyTheme, type ThemePreference, useAppliedTheme } from '@/features/appearance';

import { usePatchSettings, useSettings } from './queries';

/** The saved theme, or the one on screen while the settings load. */
export function useThemePreference(): ThemePreference {
  const { data } = useSettings();
  const applied = useAppliedTheme();
  return data?.ui.theme ?? applied;
}

/** Shows a theme at once and saves it as the user's choice. */
export function useSaveTheme(): (theme: ThemePreference) => void {
  const { mutate } = usePatchSettings();
  return (theme) => {
    applyTheme(theme);
    mutate({ ui: { theme } });
  };
}
