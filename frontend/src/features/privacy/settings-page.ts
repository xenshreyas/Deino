/** Settings → Privacy & security: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Lock } from '@/ui/icons';

import { PrivacySettings } from './components/privacy-settings';

export const settingsPages = [
  {
    id: 'security',
    group: 'privacy',
    order: 10,
    icon: Lock,
    label: 'Privacy & security',
    description: 'Encrypt the library, lock Deino behind Touch ID, and protect recordings.',
    Component: PrivacySettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    security: true;
  }
}
