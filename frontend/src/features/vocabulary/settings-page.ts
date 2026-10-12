/** Settings → Vocabulary: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { SpellCheck } from '@/ui/icons';

import { VocabularySettings } from './components/vocabulary-settings';

export const settingsPages = [
  {
    id: 'vocabulary',
    group: 'capture',
    order: 40,
    icon: SpellCheck,
    label: 'Vocabulary',
    description: 'Names and terms Deino should spell your way, and replacements.',
    Component: VocabularySettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    vocabulary: true;
  }
}
