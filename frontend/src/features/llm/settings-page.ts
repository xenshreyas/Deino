/** Settings → Notes → AI and Settings → Advanced → AI per task: this feature's settings sections (REG-02, UX-11, UX-19). */
import type { SettingsPageSection } from '@/features/settings';

import { AiProviderSettings } from './components/ai-provider-settings';
import { AiSettings } from './components/ai-settings';

export const settingsSections = [
  { id: 'ai', page: 'notes', order: 10, aliases: ['models'], Component: AiProviderSettings },
  { id: 'ai-tasks', page: 'advanced', order: 10, Component: AiSettings },
] satisfies readonly SettingsPageSection[];

declare module '@/features/settings/search' {
  interface SettingsPageIds {
    ai: true;
    models: true;
    'ai-tasks': true;
  }
}
