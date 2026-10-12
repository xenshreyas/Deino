/** Settings → Notes → About you: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { ProfilePage } from './components/profile-page';

export const settingsSections = [
  { id: 'profile', page: 'notes', order: 20, Component: ProfilePage },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    profile: true;
  }
}
