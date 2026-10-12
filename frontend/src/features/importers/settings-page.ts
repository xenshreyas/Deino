/** Settings → Import: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Inbox } from '@/ui/icons';

import { ImportSettings } from './components/import-settings';

export const settingsPages = [
  {
    id: 'import',
    group: 'data',
    order: 10,
    icon: Inbox,
    label: 'Import',
    description: 'Meetings from Granola, Circleback, Fireflies, Fathom or a folder.',
    Component: ImportSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    import: true;
  }
}
