/** Settings → Connections → Watched folders: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { WatchedFoldersSettings } from './components/watched-folders-settings';

export const settingsSections = [
  { id: 'watched-folders', page: 'connections', order: 40, Component: WatchedFoldersSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    'watched-folders': true;
  }
}
