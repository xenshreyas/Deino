/** Settings → Developer: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Wrench } from '@/ui/icons';

import { DeveloperSettings } from './components/developer-settings';

export const settingsPages = [
  {
    id: 'developer',
    group: 'more',
    order: 30,
    icon: Wrench,
    label: 'Developer',
    description: 'The deino command-line tool, and Claude Code access to your meetings.',
    Component: DeveloperSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    developer: true;
  }
}
