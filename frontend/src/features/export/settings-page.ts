/** Settings → Export: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Download } from '@/ui/icons';

import { ExportSettingsPage } from './components/export-settings';

export const settingsPages = [
  {
    id: 'export',
    group: 'data',
    order: 20,
    icon: Download,
    label: 'Export',
    description: 'Notes into a folder after every meeting, and every meeting at once.',
    Component: ExportSettingsPage,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    export: true;
  }
}
