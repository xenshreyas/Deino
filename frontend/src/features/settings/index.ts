/**
 * The settings feature: every user preference (`useSettings`) and the API
 * keys in the Keychain, which the UI can save, check and forget but never
 * read (`useSecretStatus`, `useSaveSecret`, `useDeleteSecret`).
 *
 * Mount `SettingsSync` once so the cache follows every saved change, and
 * the saved theme is applied to the app. Also the settings screen: nine
 * host pages (`SettingsPage`) that features place sections on
 * (`SettingsPageSection`) from their own `settings-page.tsx` (REG-02,
 * UX-19); routes import its URL contract from `./search`.
 */
export { secretsApi, settingsApi } from './api';
export { ApiKeyStatus } from './components/api-key-status';
export { PermissionLink, type PermissionLinkProps } from './components/permission-link';
export { SettingsSync } from './components/settings-sync';
export { apiKeyPlaceholder } from './copy';
export {
  type ModelSelection,
  modelSelectionPatch,
  PROVIDER_KEYS,
  typedKeyOf,
  useSaveModelSelection,
} from './model-selection';
export {
  secretStatusQuery,
  settingsKeys,
  settingsQuery,
  useDeleteSecret,
  usePatchSettings,
  useSaveSecret,
  useSecretStatus,
  useSettings,
  useSettingsChanges,
} from './queries';
export { SettingsScreen, type SettingsScreenProps } from './settings-screen';
export type { SettingsPage, SettingsPageSection } from './sections';
export { useSaveTheme, useThemePreference } from './theme';
export type {
  AiSettings,
  CustomOpenAiSettings,
  LlmProvider,
  NotificationSettings,
  SecretName,
  SecretStatus,
  Settings,
  SettingsPatch,
  Theme,
  TranscriptionEngine,
  TranscriptionSettings,
} from './types';
