/** Settings → Setup: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Rocket } from '@/ui/icons';

import { ResumeSetup } from './components/resume-setup';

export const settingsPages = [
  {
    id: 'setup',
    group: 'app',
    order: 40,
    icon: Rocket,
    label: 'Setup',
    description: 'Go back through first-run setup: permissions, AI, calendar and profile.',
    Component: ResumeSetup,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    setup: true;
  }
}
