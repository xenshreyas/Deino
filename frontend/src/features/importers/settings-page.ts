/** Settings → Connections → Import: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { ImportSettings } from './components/import-settings';

export const settingsSections = [
  { id: 'import', page: 'connections', order: 30, Component: ImportSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    import: true;
  }
}
