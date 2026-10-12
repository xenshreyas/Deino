/** Settings → Search index: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Search } from '@/ui/icons';

import { IndexSettings } from './components/index-settings';

export const settingsPages = [
  {
    id: 'search',
    group: 'ai',
    order: 50,
    icon: Search,
    label: 'Search index',
    description: 'Find meetings by meaning with a small model that runs on this Mac.',
    Component: IndexSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    search: true;
  }
}
