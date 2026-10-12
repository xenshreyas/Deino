/** Settings → Advanced → Search: this feature's settings section (REG-02, UX-16, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { IndexSettings } from './components/index-settings';

export const settingsSections = [
  { id: 'search', page: 'advanced', order: 30, Component: IndexSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    search: true;
  }
}
