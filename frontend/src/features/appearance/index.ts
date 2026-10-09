/**
 * Appearance: the light / dark / system theme applied to the document and
 * the native window, the first-paint cache, and the picker for Settings →
 * Appearance. The settings feature saves the choice (`ui.theme`).
 */
export { appearanceCopy } from './copy';
export { ThemePicker } from './components/theme-picker';
export { parseThemePreference, resolveTheme, type ThemePreference } from './theme';
export { THEME_STORAGE_KEY } from './theme-store';
export { applyTheme, bootTheme, useAppliedTheme } from './use-theme';
