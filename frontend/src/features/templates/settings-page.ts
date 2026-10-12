/** Settings → Templates: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { TemplatesPage } from './components/templates-page';

export const settingsSections = [
  { id: 'templates', page: 'templates', order: 10, Component: TemplatesPage },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    templates: true;
  }
}
