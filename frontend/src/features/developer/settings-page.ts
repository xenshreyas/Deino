/** Settings → Advanced → Developer: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { DeveloperSettings } from './components/developer-settings';

export const settingsSections = [
  { id: 'developer', page: 'advanced', order: 70, Component: DeveloperSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    developer: true;
  }
}
