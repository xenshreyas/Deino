/** Settings → Recording → Tell people: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { ConsentSettings } from './components/consent-settings';

export const settingsSections = [
  { id: 'consent', page: 'recording', order: 50, Component: ConsentSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    consent: true;
  }
}
