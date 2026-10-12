/** Settings → People: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Users } from '@/ui/icons';

import { PeopleSettings } from './components/people-settings';

export const settingsPages = [
  {
    id: 'people',
    group: 'capture',
    order: 60,
    icon: Users,
    label: 'People',
    description: 'Recognise speakers by voice, and the voice profiles Deino knows.',
    Component: PeopleSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    people: true;
  }
}
