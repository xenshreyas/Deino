/** Settings → Profile: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { UserRound } from '@/ui/icons';

import { ProfilePage } from './components/profile-page';

export const settingsPages = [
  {
    id: 'profile',
    group: 'ai',
    order: 40,
    icon: UserRound,
    label: 'Profile',
    description: 'Who you are and how you like notes written, for every AI prompt.',
    Component: ProfilePage,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    profile: true;
  }
}
