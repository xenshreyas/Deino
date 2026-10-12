/** Settings → Templates: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { LayoutTemplate } from '@/ui/icons';

import { TemplatesPage } from './components/templates-page';

export const settingsPages = [
  {
    id: 'templates',
    group: 'ai',
    order: 30,
    icon: LayoutTemplate,
    label: 'Templates',
    description: 'What your notes contain for each kind of meeting, and when each is used.',
    Component: TemplatesPage,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    templates: true;
  }
}
