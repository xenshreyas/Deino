/** Settings → AI models: this feature's settings page (REG-02). */
import type { SettingsPage } from '@/features/settings';
import { Sparkles } from '@/ui/icons';

import { AiSettings } from './components/ai-settings';

export const settingsPages = [
  {
    id: 'ai',
    group: 'ai',
    order: 10,
    icon: Sparkles,
    label: 'AI models',
    description: 'Which provider and model serve each AI task, and a quick test for each.',
    Component: AiSettings,
  },
] satisfies readonly SettingsPage[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    ai: true;
  }
}
