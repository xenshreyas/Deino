/** Settings → Connections → Auto-export and Export everything: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { ExportSettingsPage } from './components/export-settings';

export const settingsSections = [
  { id: 'export', page: 'connections', order: 20, Component: ExportSettingsPage },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    export: true;
  }
}
