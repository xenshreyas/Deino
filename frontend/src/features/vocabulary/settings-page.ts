/** Settings → Notes → Spelling: this feature's settings section (REG-02, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { VocabularySettings } from './components/vocabulary-settings';

export const settingsSections = [
  { id: 'vocabulary', page: 'notes', order: 30, Component: VocabularySettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    vocabulary: true;
  }
}
