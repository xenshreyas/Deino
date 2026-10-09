/**
 * The theme preference and how it is applied to the document.
 *
 * `system` follows macOS (`prefers-color-scheme`); `light` and `dark` pin the
 * theme with `data-theme` on `<html>`, which `styles/tokens.css` reads.
 * `public/theme-boot.js` repeats {@link applyThemeToDocument} before React
 * loads, so the first paint is already in the right theme.
 */
import { z } from 'zod';

/** The three choices in Settings → Appearance. */
export const themePreferenceSchema = z.enum(['system', 'light', 'dark']);

/** The user's theme choice; `system` is the default. */
export type ThemePreference = z.infer<typeof themePreferenceSchema>;

/** The theme actually on screen. */
export type ResolvedTheme = 'light' | 'dark';

/** The default for a fresh install and for unreadable stored values. */
export const DEFAULT_THEME: ThemePreference = 'system';

/** Reads any stored value as a preference, falling back to the default. */
export function parseThemePreference(raw: unknown): ThemePreference {
  const parsed = themePreferenceSchema.safeParse(raw);
  return parsed.success ? parsed.data : DEFAULT_THEME;
}

/** The theme that `preference` shows when the system is `systemDark`. */
export function resolveTheme(preference: ThemePreference, systemDark: boolean): ResolvedTheme {
  if (preference === 'system') return systemDark ? 'dark' : 'light';
  return preference;
}

/** Sets or clears `data-theme` on `root` so the tokens switch. */
export function applyThemeToDocument(preference: ThemePreference, root: HTMLElement): void {
  if (preference === 'system') {
    delete root.dataset.theme;
    return;
  }
  root.dataset.theme = preference;
}
