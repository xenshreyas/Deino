/** Settings → Watched folders: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { FolderInput } from '@/ui/icons';

import { WatchedFoldersSettings } from './components/watched-folders-settings';

export const settingsPages = [
  {
    id: 'watched-folders',
    group: 'capture',
    order: 50,
    icon: FolderInput,
    label: 'Watched folders',
    description: 'Folders whose new recordings become meetings on their own.',
    Component: WatchedFoldersSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    'watched-folders': true;
  }
}
