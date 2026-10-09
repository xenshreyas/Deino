/**
 * The settings feature: every user preference (`useSettings`) and the API
 * keys in the Keychain, which the UI can save, check and forget but never
 * read (`useSecretStatus`, `useSaveSecret`, `useDeleteSecret`).
 *
 * Mount `SettingsSync` once so the cache follows every saved change, and
 * the saved theme is applied to the app. Also the sectioned settings screen;
 * routes import its URL contract from `./search`.
 */
export { secretsApi, settingsApi } from './api';
export { ApiKeyStatus } from './components/api-key-status';
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
