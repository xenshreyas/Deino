/** Settings → Privacy → Lock, encryption and retention: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { PrivacySettings } from './components/privacy-settings';

export const settingsSections = [
  { id: 'security', page: 'privacy', order: 10, Component: PrivacySettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    security: true;
  }
}
