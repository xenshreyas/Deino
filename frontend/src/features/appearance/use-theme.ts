/**
 * The theme on screen, as shared UI state (a small Zustand store), and the
 * one function that changes it: apply to the document, sync the native
 * window, and refresh the first-paint cache.
 *
 * Saving the choice is the settings feature's job (`ui.theme` in the typed
 * settings store); it calls {@link applyTheme} and keeps the screen in step
 * with the saved value.
 */
import { create } from 'zustand';

import { applyThemeToDocument, type ThemePreference } from './theme';
import { browserThemeStore, type ThemePreferenceStore } from './theme-store';
import { syncWindowTheme } from './window-theme';

interface ThemeState {
  /** The theme on screen. */
  preference: ThemePreference;
  apply: (preference: ThemePreference) => void;
}

/** Builds the theme store over a first-paint cache (tests pass an in-memory one). */
export function createThemeStore(cache: ThemePreferenceStore) {
  return create<ThemeState>()((set) => ({
    preference: cache.load(),
    apply: (preference) => {
      cache.save(preference);
      applyThemeToDocument(preference, document.documentElement);
      void syncWindowTheme(preference);
      set({ preference });
    },
  }));
}

/** The app's theme state. */
export const useThemeStore = createThemeStore(browserThemeStore);

/** The theme on screen. */
export function useAppliedTheme(): ThemePreference {
  return useThemeStore((state) => state.preference);
}

/** Shows `preference` now: document, native window and first-paint cache. */
export function applyTheme(preference: ThemePreference): void {
  useThemeStore.getState().apply(preference);
}

/**
 * Applies the cached theme at startup: the boot script already set
 * `data-theme`, so this re-applies it (harmless) and syncs the native window.
 * The saved setting replaces it once the settings load.
 */
export function bootTheme(): void {
  const { preference } = useThemeStore.getState();
  applyThemeToDocument(preference, document.documentElement);
  void syncWindowTheme(preference);
}
