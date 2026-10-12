/** Settings → Speaker tags: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Contact } from '@/ui/icons';

import { SpeakerTagsSettings } from './components/speaker-tags-settings';

export const settingsPages = [
  {
    id: 'speaker-tags',
    group: 'capture',
    order: 70,
    icon: Contact,
    label: 'Speaker tags',
    description: 'Name speakers from Zoom, Teams and Meet, read from the call window on this Mac.',
    Component: SpeakerTagsSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    'speaker-tags': true;
  }
}
