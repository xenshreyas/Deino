/** Settings → Consent: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Megaphone } from '@/ui/icons';

import { ConsentSettings } from './components/consent-settings';

export const settingsPages = [
  {
    id: 'consent',
    group: 'capture',
    order: 80,
    icon: Megaphone,
    label: 'Consent',
    description: 'A reminder to tell people, and an automatic message in the meeting chat.',
    Component: ConsentSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    consent: true;
  }
}
