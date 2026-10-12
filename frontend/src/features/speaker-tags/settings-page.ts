/** Settings → Recording → Speaker names from call apps: this feature's settings section (REG-02, UX-09, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { SpeakerTagsSettings } from './components/speaker-tags-settings';

export const settingsSections = [
  { id: 'speaker-tags', page: 'recording', order: 80, Component: SpeakerTagsSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    'speaker-tags': true;
  }
}
