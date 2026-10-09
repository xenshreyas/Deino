/**
 * The first-paint cache of the theme.
 *
 * The saved choice lives in the typed settings store (`ui.theme`, FND-06),
 * which loads over IPC after React starts. To paint the first frame in the
 * right theme, the last applied theme is also kept in the webview's
 * `localStorage`, which `public/theme-boot.js` reads synchronously before any
 * CSS paints. The UI talks to the small {@link ThemePreferenceStore} port.
 */
import { DEFAULT_THEME, parseThemePreference, type ThemePreference } from './theme';

/** The `localStorage` key; `public/theme-boot.js` reads the same key. */
export const THEME_STORAGE_KEY = 'deino.theme';

/** Loads and saves the cached theme. */
export interface ThemePreferenceStore {
  /** The cached theme, or the default when nothing valid is cached. */
  load: () => ThemePreference;
  /** Caches the theme; failures are logged, never thrown. */
  save: (preference: ThemePreference) => void;
}

/** A cache over a Web Storage area (`localStorage` in the app). */
export function webStorageThemeStore(storage: () => Storage): ThemePreferenceStore {
  return {
    load: () => {
      try {
        return parseThemePreference(storage().getItem(THEME_STORAGE_KEY));
      } catch {
        return DEFAULT_THEME;
      }
    },
    save: (preference) => {
      try {
        storage().setItem(THEME_STORAGE_KEY, preference);
      } catch (error: unknown) {
        console.error('[appearance] Could not cache the theme', error);
      }
    },
  };
}

/** The app's first-paint cache. */
export const browserThemeStore = webStorageThemeStore(() => window.localStorage);
